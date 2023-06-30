use crate::runtime::error::RuntimeError;
use deno_core::error::AnyError;
use deno_core::resolve_path;
use deno_core::serde_v8::from_v8;
use deno_core::v8::Local;
use deno_core::FsModuleLoader;
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::worker::MainWorker;
use deno_runtime::worker::WorkerOptions;
use deno_runtime::BootstrapOptions;
use deno_runtime::WorkerLogLevel;
use log::error;
use serde_json::Value;
use std::env::current_dir;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_os = "macos")]
use super::macos::extensions::setup_extensions;

#[cfg(target_os = "windows")]
use super::windows::extensions::setup_extensions;

#[cfg(target_os = "linux")]
use super::linux::extensions::setup_extensions;

#[tokio::main]
/// Execute the decoded Javascript and return a serde_json Value
pub(crate) async fn run_script(script: &str, args: &[String]) -> Result<Value, AnyError> {
    let options = create_worker_options(args)?;
    // We already have the script data, so create a dummy path
    let uri_result = resolve_path("", &current_dir()?);
    let dummy_uri = match uri_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create dummy URI: {err:?}");
            return Err(RuntimeError::CreateUri.into());
        }
    };
    let permissions = PermissionsContainer::allow_all();
    let mut worker = MainWorker::bootstrap_from_options(dummy_uri, permissions, options);

    // Need Convert script string into a FastString: https://docs.rs/deno_core/0.180.0/deno_core/enum.FastString.html
    let script_result = worker.execute_script("deno", script.to_string().into());
    let script_output = match script_result {
        Ok(result) => result,
        Err(err) => {
            panic!("[runtime] Could not execute script: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let mut scope = worker.js_runtime.handle_scope();
    let local = Local::new(&mut scope, script_output);
    let value_result = from_v8::<Value>(&mut scope, local);
    let script_value = match value_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not get script result: {err:?}");
            return Err(RuntimeError::ScriptResult.into());
        }
    };
    Ok(script_value)
}

/// Handle Javascript errors
fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e)
        .unwrap_or("[runtime] script execution class error")
}

/// Create the Deno runtime worker options. Pass optional args
fn create_worker_options(optional_args: &[String]) -> Result<WorkerOptions, AnyError> {
    let module_loader = Rc::new(FsModuleLoader);

    let create_web_worker_cb = Arc::new(|_| {
        error!("[runtime] cannot create web workers. We are going to panic now :(");
        unreachable!("cannot create web workers")
    });

    let web_worker_event_cb = Arc::new(|_| {
        error!("[runtime] cannot create web worker event. We are going to panic now :(");
        unreachable!("cannot create web worker event")
    });

    let options = WorkerOptions {
        bootstrap: BootstrapOptions {
            args: optional_args.to_vec(),
            cpu_count: 1,
            log_level: WorkerLogLevel::Warn,
            enable_testing_features: false,
            locale: deno_core::v8::icu::get_language_tag(),
            location: None,
            no_color: false,
            is_tty: false,
            runtime_version: "0.91.0".to_string(),
            ts_version: "4.9.4".to_string(),
            unstable: false,
            user_agent: "artemis-core".to_string(),
            inspect: false,
        },
        extensions: setup_extensions(), // Register Artemis functions
        startup_snapshot: None,
        unsafely_ignore_certificate_errors: None,
        seed: None,
        source_map_getter: None,
        format_js_error_fn: None,
        web_worker_preload_module_cb: web_worker_event_cb.clone(),
        web_worker_pre_execute_module_cb: web_worker_event_cb,
        create_web_worker_cb,
        maybe_inspector_server: None,
        should_break_on_first_statement: false,
        should_wait_for_inspector_session: false,
        module_loader,
        npm_resolver: None,
        get_error_class_fn: Some(&get_error_class_name),
        cache_storage_dir: None,
        origin_storage_dir: None,
        blob_store: BlobStore::default(),
        broadcast_channel: InMemoryBroadcastChannel::default(),
        shared_array_buffer_store: None,
        compiled_wasm_module_store: None,
        stdio: Default::default(),
        create_params: None,
        root_cert_store_provider: None,
        ..Default::default()
    };
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::{create_worker_options, get_error_class_name, run_script};
    use crate::runtime::error::RuntimeError;

    #[test]
    fn test_create_worker_options() {
        let results = create_worker_options(&[]).unwrap();
        assert_eq!(results.extensions.len(), 1)
    }

    #[test]
    fn test_run_script() {
        let results = run_script("console.log('hello rust!')", &[]).unwrap();
        assert_eq!(results.is_null(), true)
    }

    #[test]
    fn test_get_error_class_name() {
        let err = RuntimeError::Decode;
        let results = get_error_class_name(&err.into());
        assert_eq!(results, "[runtime] script execution class error")
    }
}

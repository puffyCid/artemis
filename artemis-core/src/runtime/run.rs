use crate::runtime::error::RuntimeError;
use deno_core::error::AnyError;
use deno_core::serde_v8::from_v8;
use deno_core::v8::Local;
use deno_core::{FsModuleLoader, JsRuntime, RuntimeOptions, Snapshot};
use log::error;
use serde_json::Value;
use std::rc::Rc;

#[cfg(target_os = "macos")]
use super::macos::extensions::setup_extensions;

#[cfg(target_os = "windows")]
use super::windows::extensions::setup_extensions;

#[cfg(target_os = "linux")]
use super::linux::extensions::setup_extensions;

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

#[tokio::main]
/// Execute the decoded Javascript and return a serde_json Value
pub(crate) async fn run_script(script: &str, args: &[String]) -> Result<Value, AnyError> {
    let mut runtime = create_worker_options(args)?;

    /*
     // We already have the script data, so create a dummy path
     let uri_result = resolve_path("", &current_dir()?);
     let dummy_uri = match uri_result {
         Ok(result) => result,
         Err(err) => {
             error!("[runtime] Could not create dummy URI: {err:?}");
             return Err(RuntimeError::CreateUri.into());
         }
     };
     let id = runtime
         .load_main_module(&dummy_uri, Some(script.to_string().into()))
         .await?;
     let reciver = runtime.mod_evaluate(id);
     println!("waiting???");
     runtime.run_event_loop(false).await?;

    reciver.await?;
    println!("done?");
     return Ok(());
     */

    // Need Convert script string into a FastString: https://docs.rs/deno_core/0.180.0/deno_core/enum.FastString.html
    let script_result = runtime.execute_script("deno", script.to_string().into());
    let script_output = match script_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute script: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    // runtime.run_event_loop(false).await?;

    let mut scope = runtime.handle_scope();
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
    println!("Err: {e:?}");
    deno_core::error::get_custom_error_class(e).unwrap_or("[runtime] script execution class error")
}

/// Create the Deno runtime worker options. Pass optional args
fn create_worker_options(optional_args: &[String]) -> Result<JsRuntime, AnyError> {
    let module_loader = Rc::new(FsModuleLoader);

    let runtime = JsRuntime::new(RuntimeOptions {
        source_map_getter: None,
        get_error_class_fn: Some(&get_error_class_name),
        module_loader: Some(module_loader),
        extensions: setup_extensions(),
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        create_params: None,
        v8_platform: Default::default(),
        shared_array_buffer_store: Default::default(),
        compiled_wasm_module_store: None,
        inspector: false,
        is_main: Default::default(),
    });

    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use super::{create_worker_options, get_error_class_name, run_script};
    use crate::runtime::error::RuntimeError;

    #[test]
    fn test_create_worker_options() {
        let results = create_worker_options(&[]).unwrap();
        assert_eq!(results.extensions().len(), 2);
    }

    #[test]
    fn test_run_script() {
        let results = run_script("console.log('hello rust!')", &[]).unwrap();
        assert!(results.is_null());
    }

    #[test]
    fn test_get_error_class_name() {
        let err = RuntimeError::Decode;
        let results = get_error_class_name(&err.into());
        assert_eq!(results, "[runtime] script execution class error");
    }
}

use super::linux::extensions::setup_linux_extensions;
use super::macos::extensions::setup_macos_extensions;
use super::windows::extensions::setup_windows_extensions;
use crate::runtime::error::RuntimeError;
use deno_core::error::{custom_error, AnyError, JsError};
use deno_core::serde_v8::from_v8;
use deno_core::v8::{CreateParams, Local};
use deno_core::{FsModuleLoader, JsRuntime, PollEventLoopOptions, RuntimeOptions};
use log::error;
use serde_json::{json, Value};
use std::rc::Rc;

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

#[tokio::main]
/// Execute the decoded Javascript and return a `serde_json` Value
pub(crate) async fn run_script(script: &str, args: &[String]) -> Result<Value, AnyError> {
    let mut runtime = create_worker_options()?;

    // Scripts executed via `execute_script` are run in a global context.
    let scripts_args = format!("const STATIC_ARGS = {args:?}");
    let _ = runtime.execute_script("script_args", scripts_args)?;

    let script_result = runtime.execute_script("deno", script.to_string());
    let script_output = match script_result {
        Ok(result) => result,
        Err(err) => {
            let js_error = JsError {
                name: Some(String::from("ExecutionFailure")),
                message: Some(String::from("Failed to run JS code")),
                stack: None,
                cause: None,
                exception_message: err.to_string(),
                frames: Vec::new(),
                source_line: None,
                source_line_frame_index: None,
                aggregated: None,
            };
            error!("[runtime] Could not execute script: {err:?}");
            let value_error = json!(js_error);
            // Instead of erroring in Rust and cancelling the script. Let JavaScript handle the errors
            return Ok(value_error);
        }
    };

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

#[tokio::main(flavor = "current_thread")]
/// Execute the decoded async Javascript and return the data asynchronously
pub(crate) async fn run_async_script(script: &str, args: &[String]) -> Result<Value, AnyError> {
    let mut runtime = create_worker_options()?;

    // Scripts executed via `execute_script` are run in a global context.
    let scripts_args = format!("const STATIC_ARGS = {args:?}");
    let _ = runtime.execute_script("script_args", scripts_args)?;

    let script_result = runtime.execute_script("deno", script.to_string());
    let script_output = match script_result {
        Ok(result) => result,
        Err(err) => {
            let js_error = JsError {
                name: Some(String::from("ExecutionFailure")),
                message: Some(String::from("Failed to run JS code")),
                stack: None,
                cause: None,
                exception_message: err.to_string(),
                frames: Vec::new(),
                source_line: None,
                source_line_frame_index: None,
                aggregated: None,
            };
            error!("[runtime] Could not execute script: {err:?}");
            let value_error = json!(js_error);
            // Instead of erroring in Rust and cancelling the script. Let JavaScript handle the errors
            return Ok(value_error);
        }
    };
    let resolve = runtime.resolve(script_output);
    let value_result = runtime
        .with_event_loop_promise(resolve, PollEventLoopOptions::default())
        .await;

    // Wait for async script to return any value
    let value = match value_result {
        Ok(result) => result,
        Err(err) => {
            let js_error = JsError {
                name: Some(String::from("ExecutionFailure")),
                message: Some(String::from("Failed to resolve JS code")),
                stack: None,
                cause: None,
                exception_message: err.to_string(),
                frames: Vec::new(),
                source_line: None,
                source_line_frame_index: None,
                aggregated: None,
            };
            error!("[runtime] Could not resolve script: {err:?}");
            let value_error = Value::from(js_error.to_string());
            // Instead of erroring in Rust and cancelling the script. Send the error back to the JavaScript
            return Ok(value_error);
        }
    };

    let mut scope = runtime.handle_scope();
    let local = Local::new(&mut scope, value);
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
    let err = custom_error("Error", e.to_string());
    deno_core::error::get_custom_error_class(&err)
        .unwrap_or("[runtime] script execution class error")
}

/// Create the Deno runtime worker options. Pass optional args
fn create_worker_options() -> Result<JsRuntime, AnyError> {
    // This may be required for Linux? Not 100% sure. It runs fine without it. Ref: https://github.com/denoland/deno/pull/20495. May depend on V8 version (rusty_v8)
    //JsRuntime::init_platform(None);

    let module_loader = Rc::new(FsModuleLoader);

    let mut v8_params = CreateParams::default();
    let initial_size = 0;
    let max_size = 1024 * 1024 * 1024 * 2;
    // Set max heap memory size to 2GB
    v8_params = v8_params.heap_limits(initial_size, max_size);

    let mut extensions;

    extensions = setup_macos_extensions();
    extensions.append(&mut setup_linux_extensions());
    extensions.append(&mut setup_windows_extensions());

    let runtime = JsRuntime::new(RuntimeOptions {
        source_map_getter: None,
        get_error_class_fn: Some(&get_error_class_name),
        module_loader: Some(module_loader),
        extensions,
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        create_params: Some(v8_params),
        v8_platform: Default::default(),
        shared_array_buffer_store: Default::default(),
        compiled_wasm_module_store: None,
        inspector: false,
        is_main: Default::default(),
        op_metrics_factory_fn: None,
        feature_checker: None,
        skip_op_registration: false,
        validate_import_attributes_cb: Default::default(),
        import_meta_resolve_callback: Default::default(),
        wait_for_inspector_disconnect_callback: None,
        custom_module_evaluation_cb: None,
        extension_transpiler: None,
        enable_code_cache: false,
        eval_context_code_cache_cbs: None,
    });

    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use super::{create_worker_options, get_error_class_name, run_script};
    use crate::runtime::{error::RuntimeError, run::run_async_script};

    #[test]
    fn test_create_worker_options() {
        let results = create_worker_options().unwrap();
        assert!(results.op_names().len() > 2);
    }

    #[test]
    fn test_run_script() {
        let results = run_script("console.log('hello rust!')", &[]).unwrap();
        assert!(results.is_null());
    }

    #[test]
    fn test_run_async_script() {
        let results = run_async_script("console.error('hello async rust!')", &[]).unwrap();
        assert!(results.is_null());
    }

    #[test]
    fn test_get_error_class_name() {
        let err = RuntimeError::Decode;
        let results = get_error_class_name(&err.into());
        assert_eq!(results, "Error");
    }
}

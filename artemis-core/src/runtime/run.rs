use crate::runtime::error::RuntimeError;
use deno_core::error::{AnyError, JsError};
use deno_core::serde_v8::from_v8;
use deno_core::v8::{CreateParams, Local};
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
    let mut runtime = create_worker_options()?;

    // Scripts executed via `execute_script` are run in a global context.
    let scripts_args = format!("const STATIC_ARGS = {args:?}");
    let _ = runtime.execute_script("script_args", scripts_args.into())?;

    // Need Convert script string into a FastString: https://docs.rs/deno_core/0.180.0/deno_core/enum.FastString.html
    let script_result = runtime.execute_script("deno", script.to_string().into());
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
            let value_error = Value::from(js_error.to_string());
            // Instead of erroring in Rust and cancelling the script. Send the error back to the JavaScript
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

#[tokio::main]
/// Execute the decoded async Javascript and return the data asynchronously
pub(crate) async fn run_async_script(script: &str, args: &[String]) -> Result<Value, AnyError> {
    let mut runtime = create_worker_options()?;

    // Scripts executed via `execute_script` are run in a global context.
    let scripts_args = format!("const STATIC_ARGS = {args:?}");
    let _ = runtime.execute_script("script_args", scripts_args.into())?;

    let script_result = runtime.execute_script("deno", script.to_string().into());
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
            let value_error = Value::from(js_error.to_string());
            // Instead of erroring in Rust and cancelling the script. Send the error back to the JavaScript
            return Ok(value_error);
        }
    };
    // Wait for async script to return any value
    let value = runtime.resolve_value(script_output).await?;

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
    deno_core::error::get_custom_error_class(e).unwrap_or("[runtime] script execution class error")
}

/// Create the Deno runtime worker options. Pass optional args
fn create_worker_options() -> Result<JsRuntime, AnyError> {
    let module_loader = Rc::new(FsModuleLoader);

    let mut v8_params = CreateParams::default();
    let initial_size = 0;
    let max_size = 1024 * 1024 * 1024 * 2;
    // Set max heap memory size to 2GB
    v8_params = v8_params.heap_limits(initial_size, max_size);

    let runtime = JsRuntime::new(RuntimeOptions {
        source_map_getter: None,
        get_error_class_fn: Some(&get_error_class_name),
        module_loader: Some(module_loader),
        extensions: setup_extensions(),
        startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
        create_params: Some(v8_params),
        v8_platform: Default::default(),
        shared_array_buffer_store: Default::default(),
        compiled_wasm_module_store: None,
        inspector: false,
        is_main: Default::default(),
        preserve_snapshotted_modules: None,
        op_metrics_factory_fn: None,
        feature_checker: None,
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
        assert_eq!(results.extensions().len(), 2);
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
        assert_eq!(results, "[runtime] script execution class error");
    }
}

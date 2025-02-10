use super::{
    error::RuntimeError, filesystem::extensions::filesystem_functions,
    nom::extensions::nom_functions,
};
use boa_engine::{property::Attribute, Context, Source};
use boa_runtime::Console;
use serde_json::Value;

pub(crate) fn run_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    let mut context = Context::default();

    let console = Console::init(&mut context);
    let status = context.register_global_property(Console::NAME, console, Attribute::all());
    if status.is_err() {
        panic!(
            "[runtime] Could not register console property: {:?}",
            status.unwrap_err()
        );
        return Err(RuntimeError::ExecuteScript);
    }

    nom_functions(&mut context);
    filesystem_functions(&mut context);

    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(err) => {
            panic!("[runtime] Could not execute script: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };
    if result.is_undefined() {
        return Ok(Value::Null);
    }
    let value = match result.to_json(&mut context) {
        Ok(result) => result,
        Err(err) => {
            panic!("[runtime] Could not serialize script value: {err:?}");
            return Err(RuntimeError::ScriptResult);
        }
    };

    Ok(value)
}

pub(crate) fn run_async_script(script: &str, args: &[String]) -> Result<Value, RuntimeError> {
    Ok(Value::Null)
}

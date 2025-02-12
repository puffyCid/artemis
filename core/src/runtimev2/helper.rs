use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};
use serde_json::Value;

pub(crate) fn string_arg(args: &[JsValue], index: &usize) -> JsResult<String> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_string() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not a string").into(),
        ));
    }

    // Unwrap is ok since we checked above to make sure its a string
    let value = match arg_value.as_string().unwrap().to_std_string() {
        Ok(result) => result,
        Err(err) => {
            return Err(JsError::from_opaque(
                js_string!("Could not extract string").into(),
            ));
        }
    };

    Ok(value)
}

pub(crate) fn bool_arg(args: &[JsValue], index: &usize) -> JsResult<bool> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_boolean() {
        return Err(JsError::from_opaque(js_string!("Arg is not a bool").into()));
    }

    // Unwrap is ok since we checked above to make sure its a bool
    let value = arg_value.as_boolean().unwrap();

    Ok(value)
}

pub(crate) fn value_arg(args: &[JsValue], index: &usize, context: &mut Context) -> JsResult<Value> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_object() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not an object").into(),
        ));
    }

    // Unwrap is ok since we checked above to make sure its an object
    let value = arg_value.to_json(context).unwrap();

    Ok(value)
}

pub(crate) fn bytes_arg(
    args: &[JsValue],
    index: &usize,
    context: &mut Context,
) -> JsResult<Vec<u8>> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_object() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not an object").into(),
        ));
    }

    // Unwrap is ok since we checked above to make sure its an object
    let value = arg_value.to_json(context).unwrap();
    let bytes = serde_json::to_vec(&value).unwrap_or_default();

    Ok(bytes)
}

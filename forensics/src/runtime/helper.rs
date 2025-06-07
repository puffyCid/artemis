use boa_engine::{
    Context, JsArgs, JsError, JsResult, JsValue, js_string,
    object::builtins::{JsArrayBuffer, JsUint8Array},
};
use serde_json::Value;

/// Get the JS argument and convert to string
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
            let issue = format!("Could not extract string: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    Ok(value)
}

/// Get the JS argument and convert to char
pub(crate) fn char_arg(args: &[JsValue], index: &usize) -> JsResult<char> {
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
            let issue = format!("Could not extract string: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    Ok(value.chars().next().unwrap_or_default())
}

/// Get the JS argument and convert to number
pub(crate) fn number_arg(args: &[JsValue], index: &usize) -> JsResult<f64> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_number() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not a number").into(),
        ));
    }

    // Unwrap is ok since we checked above to make sure its a number
    let value = arg_value.as_number().unwrap();

    Ok(value)
}

/// Get the JS argument and convert to big number
pub(crate) fn bigint_arg(args: &[JsValue], index: &usize) -> JsResult<f64> {
    let arg_value = args.get_or_undefined(*index);
    if arg_value.is_bigint() {
        // Unwrap is ok since we checked above to make sure its a number
        let value = arg_value.as_bigint().unwrap().to_f64();

        return Ok(value);
    } else if arg_value.is_number() {
        return number_arg(args, index);
    }

    Err(JsError::from_opaque(
        js_string!("Arg is not a bigint nor a number").into(),
    ))
}

/// Get the JS argument and convert to boolean
pub(crate) fn bool_arg(args: &[JsValue], index: &usize) -> JsResult<bool> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_boolean() {
        return Err(JsError::from_opaque(js_string!("Arg is not a bool").into()));
    }

    let value = arg_value.to_boolean();

    Ok(value)
}

/// Get the JS argument and convert to object
pub(crate) fn value_arg(args: &[JsValue], index: &usize, context: &mut Context) -> JsResult<Value> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_object() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not an object").into(),
        ));
    }

    let value = arg_value.to_json(context)?;

    Ok(value)
}

/// Get the JS argument and convert to bytes
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

    let arr = JsUint8Array::from_object(arg_value.to_object(context)?)?;
    let value = arr.buffer(context)?;
    let buff = JsArrayBuffer::from_object(value.to_object(context)?)?;
    let bytes = match buff.data() {
        Some(result) => result,
        None => {
            return Err(JsError::from_opaque(
                js_string!("Buffer is detached").into(),
            ));
        }
    };
    Ok(bytes.to_vec())
}

/// Get the JS argument and convert to boolean
pub(crate) fn boolean_arg(
    args: &[JsValue],
    index: &usize,
    _context: &mut Context,
) -> JsResult<bool> {
    let arg_value = args.get_or_undefined(*index);
    if !arg_value.is_boolean() {
        return Err(JsError::from_opaque(
            js_string!("Arg is not a boolean").into(),
        ));
    }

    let value = arg_value.to_boolean();
    Ok(value)
}

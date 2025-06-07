use crate::{
    artifacts::os::macos::accounts::{groups::grab_groups, users::grab_users},
    runtime::helper::string_arg,
    structs::artifacts::os::macos::{MacosGroupsOptions, MacosUsersOptions},
};
use boa_engine::{Context, JsArgs, JsResult, JsValue};

/// Expose parsing Users to `BoaJS`
pub(crate) fn js_users_macos(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };

    let users = grab_users(&MacosUsersOptions { alt_path: path });
    let results = serde_json::to_value(&users).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing Groups to `BoaJS`
pub(crate) fn js_groups_macos(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };
    let groups = grab_groups(&MacosGroupsOptions { alt_path: path });

    let results = serde_json::to_value(&groups).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_js_users_groups() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNlcnMoKSB7CiAgICBjb25zdCBkYXRhID0ganNfdXNlcnNfbWFjb3MoKTsKICAgIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGdldF9ncm91cHMoKSB7CiAgICBjb25zdCBkYXRhID0ganNfZ3JvdXBzX21hY29zKCk7CiAgICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBnZXRVc2VycygpIHsKICAgIHJldHVybiBnZXRfdXNlcnMoKTsKfQpmdW5jdGlvbiBnZXRHcm91cHMoKSB7CiAgICByZXR1cm4gZ2V0X2dyb3VwcygpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCB1c2VycyA9IGdldFVzZXJzKCk7CiAgICBjb25zdCBncm91cHMgPSBnZXRHcm91cHMoKTsKICAgIGNvbnN0IGFjY291bnRzID0gewogICAgICAgIHVzZXJzLAogICAgICAgIGdyb3VwcwogICAgfTsKICAgIHJldHVybiBhY2NvdW50czsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

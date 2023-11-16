use crate::{
    artifacts::os::windows::userassist::parser::grab_userassist, runtime::error::RuntimeError,
    structs::artifacts::os::windows::UserAssistOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing userassist located on systemdrive to `Deno`
pub(crate) fn get_userassist(resolve: bool) -> Result<String, AnyError> {
    let options = UserAssistOptions {
        alt_drive: None,
        resolve_descriptions: Some(resolve),
    };

    let assist = grab_userassist(&options)?;

    let results = serde_json::to_string(&assist)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing userassist located on alt drive to `Deno`
pub(crate) fn get_alt_userassist(
    #[string] drive: String,
    resolve: bool,
) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt userassist drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = UserAssistOptions {
        alt_drive: Some(drive_char),
        resolve_descriptions: Some(resolve),
    };

    let assist = grab_userassist(&options)?;

    let results = serde_json::to_string(&assist)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_userassist() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNlcmFzc2lzdCgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF91c2VyYXNzaXN0KGZhbHNlKTsKICAgIGNvbnN0IGFzc2lzdF9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gYXNzaXN0X2FycmF5Owp9CmZ1bmN0aW9uIGdldFVzZXJBc3Npc3QoKSB7CiAgICByZXR1cm4gZ2V0X3VzZXJhc3Npc3QoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYXNzaXN0ID0gZ2V0VXNlckFzc2lzdCgpOwogICAgcmV0dXJuIGFzc2lzdDsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("userassist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_userassist() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3VzZXJhc3Npc3QoZHJpdmUpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfdXNlcmFzc2lzdChkcml2ZSwgZmFsc2UpOwogICAgY29uc3QgYXNzaXN0X2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBhc3Npc3RfYXJyYXk7Cn0KZnVuY3Rpb24gZ2V0VXNlckFsdEFzc2lzdChkcml2ZSkgewogICAgcmV0dXJuIGdldF9hbHRfdXNlcmFzc2lzdChkcml2ZSk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGFzc2lzdCA9IGdldFVzZXJBbHRBc3Npc3QoIkMiKTsKICAgIHJldHVybiBhc3Npc3Q7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("userassist_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

use crate::{
    artifacts::os::windows::shellbags::parser::grab_shellbags, runtime::error::RuntimeError,
    structs::artifacts::os::windows::ShellbagsOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing shellbags located on systemdrive to `Deno`
fn get_shellbags(resolve: bool) -> Result<String, AnyError> {
    let options = ShellbagsOptions {
        alt_drive: None,
        resolve_guids: resolve,
    };
    let bags_result = grab_shellbags(&options);
    let bags = match bags_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shellbags: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&bags)?;
    Ok(results)
}

#[op]
/// Expose parsing shellbags located on alt drive to `Deno`
fn get_alt_shellbags(drive: String, resolve: bool) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt shellbags drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = &drive.chars().next().unwrap();
    let options = ShellbagsOptions {
        alt_drive: Some(drive_char.to_owned()),
        resolve_guids: resolve,
    };

    let bags_result = grab_shellbags(&options);
    let bags = match bags_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse alt shellbags: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&bags)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    fn test_get_shellbags() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hlbGxiYWdzKHJlc29sdmVfZ3VpZHMpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9zaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcyk7CiAgICBjb25zdCBiYWdzX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBiYWdzX2FycmF5Owp9CmZ1bmN0aW9uIGdldFNoZWxsYmFncyhyZXNvbHZlX2d1aWRzKSB7CiAgICByZXR1cm4gZ2V0X3NoZWxsYmFncyhyZXNvbHZlX2d1aWRzKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYmFncyA9IGdldFNoZWxsYmFncyh0cnVlKTsKICAgIHJldHVybiBiYWdzOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellbags"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_shellbags() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3NoZWxsYmFncyhyZXNvbHZlX2d1aWRzLCBkcml2ZSkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3NoZWxsYmFncyhyZXNvbHZlX2d1aWRzLCBkcml2ZSk7CiAgICBjb25zdCBiYWdzX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBiYWdzX2FycmF5Owp9CmZ1bmN0aW9uIGdldEFsdFNoZWxsYmFncyhyZXNvbHZlX2d1aWRzKSB7CiAgICByZXR1cm4gZ2V0X2FsdF9zaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcyk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGJhZ3MgPSBnZXRBbHRTaGVsbGJhZ3ModHJ1ZSwgIkMiKTsKICAgIGNvbnN0IGJhZ3NfZXhjZXB0X2RpcmVjdG9yeSA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyeSBvZiBiYWdzKXsKICAgICAgICBpZiAoZW50cnkuc2hlbGxfdHlwZSA9PSAiRGlyZWN0b3J5IikgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgYmFnc19leGNlcHRfZGlyZWN0b3J5LnB1c2goZW50cnkpOwogICAgfQogICAgcmV0dXJuIGJhZ3NfZXhjZXB0X2RpcmVjdG9yeTsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellbags_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

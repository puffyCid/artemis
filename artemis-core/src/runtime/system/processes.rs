use crate::{
    artifacts::os::processes::process::proc_list, filesystem::files::Hashes,
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose pulling process listing to `Deno`
pub(crate) fn get_processes(#[string] hashes: String, metadata: bool) -> Result<String, AnyError> {
    let hashes: Hashes = serde_json::from_str(&hashes).unwrap_or(Hashes {
        md5: false,
        sha1: false,
        sha256: false,
    });
    let proc_results = proc_list(&hashes, metadata);
    let proc = match proc_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get process listing: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&proc)?;
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
    fn test_get_processes() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcHJvY2Vzc2VzKG1kNSwgc2hhMSwgc2hhMjU2LCBwZV9pbmZvKSB7CiAgICBjb25zdCBoYXNoZXMgPSB7CiAgICAgICAgbWQ1LAogICAgICAgIHNoYTEsCiAgICAgICAgc2hhMjU2CiAgICB9OwogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3Byb2Nlc3NlcyhKU09OLnN0cmluZ2lmeShoYXNoZXMpLCBwZV9pbmZvKTsKICAgIGNvbnN0IHByb2NfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIHByb2NfYXJyYXk7Cn0KZnVuY3Rpb24gZ2V0UHJvY2Vzc2VzKG1kNSwgc2hhMSwgc2hhMjU2LCBwZV9pbmZvKSB7CiAgICByZXR1cm4gZ2V0X3Byb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHByb2NfbGlzdCA9IGdldFByb2Nlc3Nlcyh0cnVlLCBmYWxzZSwgZmFsc2UsIHRydWUpOwogICAgcmV0dXJuIHByb2NfbGlzdDsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("processes"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

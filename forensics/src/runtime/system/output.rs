use crate::{
    output::{
        manager::OutputManager,
        record::{Record, SingleRecordStream, VecRecordStream},
    },
    runtime::helper::{string_arg, value_arg},
    structs::{artifacts::runtime::script::JSScript, toml::OutputConfig},
};
use boa_engine::{
    Context, JsData, JsError, JsResult, JsValue, NativeFunction,
    class::{Class, ClassBuilder},
    js_string,
};
use boa_gc::{Finalize, Trace};
use std::cell::RefCell;
use tracing::error;

/// Expose the `OutputManager` to JavaScript
#[derive(Trace, Finalize, JsData)]
pub(crate) struct JsOutputManager {
    /// Basically tells the `BoaJS` garbage collector not to touch our `OutputManager`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    manager: RefCell<Option<OutputManager>>,
}

impl Class for JsOutputManager {
    const NAME: &'static str = "JsOutputManager";
    const LENGTH: usize = 1;

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(
            js_string!("js_write_artifact"),
            2,
            NativeFunction::from_fn_ptr(Self::js_write_artifact),
        );
        class.method(
            js_string!("js_finalize"),
            0,
            NativeFunction::from_fn_ptr(Self::js_finalize),
        );

        Ok(())
    }

    fn data_constructor(
        _this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<Self> {
        let output_format = value_arg(args, 0, context)?;

        let config: OutputConfig = match serde_json::from_value(output_format) {
            Ok(results) => results,
            Err(err) => {
                error!("Failed deserialize output config format: {err:?}");
                let issue = format!("Failed deserialize output config format: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let manager = match OutputManager::new(config) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to create OutputManager: {err:?}");
                let issue = format!("Failed to create OutputManager: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let js_manager = JsOutputManager {
            manager: RefCell::new(Some(manager)),
        };

        Ok(js_manager)
    }
}

impl JsOutputManager {
    /// Write the JavaScript data to our configured `OutputManager`
    fn js_write_artifact(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let obj_manager = match this.as_object() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(js_string!("Not an Object").into()));
            }
        };
        let js_manager = match obj_manager.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a OutputManager Object").into(),
                ));
            }
        };

        let mut manager_ref = js_manager.manager.borrow_mut();
        let manager = match manager_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get OutputManager").into(),
                ));
            }
        };

        let entries = value_arg(args, 0, context)?;
        let output_name = string_arg(args, 1)?;
        let options = JSScript {
            name: output_name,
            script: String::new(),
        };
        let records = match Record::from_value(entries) {
            Ok(result) => result,
            Err(err) => {
                error!("Could not create record from data: {err:?}");
                return Err(JsError::from_opaque(
                    js_string!("Could not create record from data").into(),
                ));
            }
        };

        match records {
            Record::Array(record_array) => {
                if let Err(err) = manager.write_artifact(
                    &options.name,
                    &options,
                    &mut VecRecordStream::new(record_array),
                ) {
                    error!("Could not write record from data: {err:?}");
                    return Err(JsError::from_opaque(
                        js_string!("Could not write record from data").into(),
                    ));
                }
            }
            // All other values are treat as a `SingleRecordStream`
            _ => {
                if let Err(err) = manager.write_artifact(
                    &options.name,
                    &options,
                    &mut SingleRecordStream::new(records),
                ) {
                    error!("Could not write record from data: {err:?}");
                    return Err(JsError::from_opaque(
                        js_string!("Could not write record from data").into(),
                    ));
                }
            }
        }

        let success = true;
        Ok(JsValue::new(success))
    }

    /// Finish writing artifacts using the `OutputManager`
    fn js_finalize(this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let obj_manager = match this.as_object() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(js_string!("Not an Object").into()));
            }
        };
        let js_manager = match obj_manager.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a OutputManager Object").into(),
                ));
            }
        };

        let manager_ref = js_manager.manager.take();
        let manager = match manager_ref {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get OutputManager").into(),
                ));
            }
        };

        if let Err(err) = manager.finalize() {
            error!("Could not complete record from data: {err:?}");
            let issue = format!("Could not complete record from data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        let sucsess = true;
        Ok(JsValue::new(sucsess))
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::{
        fs::{read_dir, read_to_string},
        path::PathBuf,
    };

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_output_results() {
        let test = "Ly8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzDQp2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7DQogIG5hbWU7DQogIG1lc3NhZ2U7DQogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsNCiAgICBzdXBlcigpOw0KICAgIHRoaXMubmFtZSA9IG5hbWU7DQogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsNCiAgfQ0KfTsNCg0KLy8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9lcnJvci50cw0KdmFyIFN5c3RlbUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugew0KfTsNCg0KLy8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9vdXRwdXQudHMNCnZhciBPdXRwdXRNYW5hZ2VyID0gY2xhc3Mgew0KICBtYW5hZ2VyOw0KICAvKioNCiAgICogQ29uc3RydWN0IHRoZSBhcnRlbWlzIGBPdXRwdXRNYW5hZ2VyYA0KICAgKiBAcGFyYW0gb3V0cHV0IGBPdXRwdXRgIG9iamVjdCBzdHJ1Y3R1cmUNCiAgICovDQogIGNvbnN0cnVjdG9yKG91dHB1dDIpIHsNCiAgICB0aGlzLm1hbmFnZXIgPSBuZXcgSnNPdXRwdXRNYW5hZ2VyKG91dHB1dDIpOw0KICB9DQogIC8qKg0KICAgKiBGdW5jdGlvbiB0byB3cml0ZSBhcnRpZmFjdCBkYXRhIHJlc3VsdHMNCiAgICogQHBhcmFtIGRhdGEgQXJ0aWZhY3QgZGF0YSB0byB3cml0ZQ0KICAgKiBAcGFyYW0gYXJ0aWZhY3RfbmFtZSBOYW1lIG9mIGFydGlmYWN0IHRvIHdyaXRlIHRvDQogICAqIEByZXR1cm5zIFRydWUgb24gc3VjY2VzcyBvciBgU3lzdGVtRXJyb3JgDQogICAqLw0KICB3cml0ZV9hcnRpZmFjdChkYXRhLCBhcnRpZmFjdF9uYW1lKSB7DQogICAgdHJ5IHsNCiAgICAgIGNvbnN0IHJlc3VsdHMgPSB0aGlzLm1hbmFnZXIuanNfd3JpdGVfYXJ0aWZhY3QoZGF0YSwgYXJ0aWZhY3RfbmFtZSk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgU3lzdGVtRXJyb3IoYE9VVFBVVGAsIGBmYWlsZWQgdG8gd3JpdGUgYXJ0aWZhY3Q6ICR7ZXJyfWApOw0KICAgIH0NCiAgfQ0KICAvKioNCiAgICogRnVuY3Rpb24gdG8gZmluaXNoIHdyaXRpbmcgYXJ0aWZhY3QgcmVzdWx0cy4gT25jZSB0aGlzIGZ1bmN0aW9uIGlzIGNhbGxlZCB0aGUgYE91dHB1dE1hbmFnZXJgIGlzIGRlc3Ryb3llZCBhbmQgY2Fubm90IGJlIHVzZWQgYWdhaW4NCiAgICogQHJldHVybnMgVHJ1ZSBvbiBzdWNjZXNzIG9yIGBTeXN0ZW1FcnJvcmANCiAgICovDQogIGZpbmFsaXplKCkgew0KICAgIHRyeSB7DQogICAgICBjb25zdCByZXN1bHRzID0gdGhpcy5tYW5hZ2VyLmpzX2ZpbmFsaXplKCk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgU3lzdGVtRXJyb3IoYE9VVFBVVGAsIGBmYWlsZWQgdG8gZmluYWxpemUgb3V0cHV0OiAke2Vycn1gKTsNCiAgICB9DQogIH0NCn07DQoNCi8vIC4uLy4uLy4uL2FydGVtaXMtYXBpL3NyYy9zeXN0ZW0vbWVtb3J5LnRzDQpmdW5jdGlvbiBwcm9jZXNzTGlzdGluZyhtZDUgPSBmYWxzZSwgc2hhMSA9IGZhbHNlLCBzaGEyNTYgPSBmYWxzZSwgYmluYXJ5ID0gZmFsc2UpIHsNCiAgY29uc3QgaGFzaGVzID0gew0KICAgIG1kNSwNCiAgICBzaGExLA0KICAgIHNoYTI1Ng0KICB9Ow0KICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygNCiAgICBoYXNoZXMsDQogICAgYmluYXJ5DQogICk7DQogIHJldHVybiBkYXRhOw0KfQ0KDQovLyAuLi8uLi8uLi8uLi9Eb3dubG9hZHMvbWFpbi50cw0KZnVuY3Rpb24gbWFpbigpIHsNCiAgY29uc3Qgb3V0ID0gew0KICAgIG5hbWU6ICJhcnRlbWlzX3Byb2NfdmFsaWRhdGUiLA0KICAgIGRpcmVjdG9yeTogIi4vdG1wIiwNCiAgICBmb3JtYXQ6ICJqc29ubCIgLyogSlNPTkwgKi8sDQogICAgY29tcHJlc3M6IGZhbHNlLA0KICAgIGVuZHBvaW50X2lkOiAiIiwNCiAgICBjb2xsZWN0aW9uX2lkOiAwLA0KICAgIGRlc3RpbmF0aW9uOiAibG9jYWwiIC8qIExPQ0FMICovDQogIH07DQogIGNvbnN0IHByb2NzID0gcHJvY2Vzc0xpc3RpbmcoZmFsc2UsIGZhbHNlLCBmYWxzZSwgZmFsc2UpOw0KICBjb25zdCBtYW5hZ2VyID0gbmV3IE91dHB1dE1hbmFnZXIob3V0KTsNCiAgbWFuYWdlci53cml0ZV9hcnRpZmFjdChwcm9jcywgImpzX3Byb2NfbGlzdGluZyIpOw0KICBtYW5hZ2VyLmZpbmFsaXplKCk7DQp9DQptYWluKCk7";
        let mut output = output_options("artemis_proc_validate", "./tmp", false);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("artemis_proc_validate"));
        assert!(output_dir.exists());
        let mut jsonl_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("js_proc_listing") && name.ends_with(".jsonl") {
                jsonl_files.push(path);
            }
        }
        assert!(jsonl_files.len() >= 1);
        let text = read_to_string(&jsonl_files[0]).unwrap();
        assert!(text.contains("forensics-"));
    }

    #[test]
    fn test_output_results_json_compress() {
        let test = "Ly8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzDQp2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7DQogIG5hbWU7DQogIG1lc3NhZ2U7DQogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsNCiAgICBzdXBlcigpOw0KICAgIHRoaXMubmFtZSA9IG5hbWU7DQogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsNCiAgfQ0KfTsNCg0KLy8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9lcnJvci50cw0KdmFyIFN5c3RlbUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugew0KfTsNCg0KLy8gLi4vLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9vdXRwdXQudHMNCnZhciBPdXRwdXRNYW5hZ2VyID0gY2xhc3Mgew0KICBtYW5hZ2VyOw0KICAvKioNCiAgICogQ29uc3RydWN0IHRoZSBhcnRlbWlzIGBPdXRwdXRNYW5hZ2VyYA0KICAgKiBAcGFyYW0gb3V0cHV0IGBPdXRwdXRgIG9iamVjdCBzdHJ1Y3R1cmUNCiAgICovDQogIGNvbnN0cnVjdG9yKG91dHB1dDIpIHsNCiAgICB0aGlzLm1hbmFnZXIgPSBuZXcgSnNPdXRwdXRNYW5hZ2VyKG91dHB1dDIpOw0KICB9DQogIC8qKg0KICAgKiBGdW5jdGlvbiB0byB3cml0ZSBhcnRpZmFjdCBkYXRhIHJlc3VsdHMNCiAgICogQHBhcmFtIGRhdGEgQXJ0aWZhY3QgZGF0YSB0byB3cml0ZQ0KICAgKiBAcGFyYW0gYXJ0aWZhY3RfbmFtZSBOYW1lIG9mIGFydGlmYWN0IHRvIHdyaXRlIHRvDQogICAqIEByZXR1cm5zIFRydWUgb24gc3VjY2VzcyBvciBgU3lzdGVtRXJyb3JgDQogICAqLw0KICB3cml0ZV9hcnRpZmFjdChkYXRhLCBhcnRpZmFjdF9uYW1lKSB7DQogICAgdHJ5IHsNCiAgICAgIGNvbnN0IHJlc3VsdHMgPSB0aGlzLm1hbmFnZXIuanNfd3JpdGVfYXJ0aWZhY3QoZGF0YSwgYXJ0aWZhY3RfbmFtZSk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgU3lzdGVtRXJyb3IoYE9VVFBVVGAsIGBmYWlsZWQgdG8gd3JpdGUgYXJ0aWZhY3Q6ICR7ZXJyfWApOw0KICAgIH0NCiAgfQ0KICAvKioNCiAgICogRnVuY3Rpb24gdG8gZmluaXNoIHdyaXRpbmcgYXJ0aWZhY3QgcmVzdWx0cy4gT25jZSB0aGlzIGZ1bmN0aW9uIGlzIGNhbGxlZCB0aGUgYE91dHB1dE1hbmFnZXJgIGlzIGRlc3Ryb3llZCBhbmQgY2Fubm90IGJlIHVzZWQgYWdhaW4NCiAgICogQHJldHVybnMgVHJ1ZSBvbiBzdWNjZXNzIG9yIGBTeXN0ZW1FcnJvcmANCiAgICovDQogIGZpbmFsaXplKCkgew0KICAgIHRyeSB7DQogICAgICBjb25zdCByZXN1bHRzID0gdGhpcy5tYW5hZ2VyLmpzX2ZpbmFsaXplKCk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgU3lzdGVtRXJyb3IoYE9VVFBVVGAsIGBmYWlsZWQgdG8gZmluYWxpemUgb3V0cHV0OiAke2Vycn1gKTsNCiAgICB9DQogIH0NCn07DQoNCi8vIC4uLy4uLy4uL2FydGVtaXMtYXBpL3NyYy9zeXN0ZW0vbWVtb3J5LnRzDQpmdW5jdGlvbiBwcm9jZXNzTGlzdGluZyhtZDUgPSBmYWxzZSwgc2hhMSA9IGZhbHNlLCBzaGEyNTYgPSBmYWxzZSwgYmluYXJ5ID0gZmFsc2UpIHsNCiAgY29uc3QgaGFzaGVzID0gew0KICAgIG1kNSwNCiAgICBzaGExLA0KICAgIHNoYTI1Ng0KICB9Ow0KICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygNCiAgICBoYXNoZXMsDQogICAgYmluYXJ5DQogICk7DQogIHJldHVybiBkYXRhOw0KfQ0KDQovLyAuLi8uLi8uLi8uLi9Eb3dubG9hZHMvbWFpbi50cw0KZnVuY3Rpb24gbWFpbigpIHsNCiAgY29uc3Qgb3V0ID0gew0KICAgIG5hbWU6ICJydW50aW1lX3Rlc3QiLA0KICAgIGRpcmVjdG9yeTogIi4vdG1wIiwNCiAgICBmb3JtYXQ6ICJqc29uIiAvKiBKU09OICovLA0KICAgIGNvbXByZXNzOiB0cnVlLA0KICAgIGVuZHBvaW50X2lkOiAiIiwNCiAgICBjb2xsZWN0aW9uX2lkOiAwLA0KICAgIGRlc3RpbmF0aW9uOiAibG9jYWwiIC8qIExPQ0FMICovDQogIH07DQogIGNvbnN0IHByb2NzID0gcHJvY2Vzc0xpc3RpbmcoZmFsc2UsIGZhbHNlLCBmYWxzZSwgZmFsc2UpOw0KICBjb25zdCBtYW5hZ2VyID0gbmV3IE91dHB1dE1hbmFnZXIob3V0KTsNCiAgbWFuYWdlci53cml0ZV9hcnRpZmFjdChwcm9jcywgImpzX3Byb2NfbGlzdGluZyIpOw0KICBtYW5hZ2VyLmZpbmFsaXplKCk7DQp9DQptYWluKCk7";
        let mut output = output_options("runtime_test", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

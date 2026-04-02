use crate::{
    filesystem::ntfs::reader::read_bytes,
    runtime::helper::{number_arg, string_arg},
};
use boa_engine::{
    Context, JsData, JsError, JsResult, JsValue, NativeFunction,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsUint8Array,
};
use boa_gc::{Finalize, Trace};
use std::{cell::RefCell, fs::File, io::BufReader};

#[derive(Trace, Finalize, JsData)]
pub(crate) struct JsBufReader {
    /// Basically tells the `BoaJS` garabage collector not to touch our `BufReader`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    reader: RefCell<Option<BufReader<File>>>,
}

/// Create a simple `BufReader` class that can be used to stream files
impl Class for JsBufReader {
    const NAME: &'static str = "JsBufReader";
    const LENGTH: usize = 1;

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(
            js_string!("read"),
            2,
            NativeFunction::from_fn_ptr(Self::read),
        );
        Ok(())
    }

    /// Initial the structure of the `JsBufReader` class
    /// This is the `constructor` method
    fn data_constructor(
        _this: &JsValue,
        args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        let path = string_arg(args, 0)?;
        let file = match File::open(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not open {path}: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let reader = JsBufReader {
            reader: RefCell::new(Some(BufReader::new(file))),
        };

        Ok(reader)
    }
}

/// Here be dragons
impl JsBufReader {
    /// Read bytes from a file using standard OS APIs
    /// Must provide offset to start and how many bytes to read
    fn read(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let obj_reader = match this.as_object() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(js_string!("Not an Object").into()));
            }
        };
        let js_reader = match obj_reader.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a FileReader Object").into(),
                ));
            }
        };

        let mut reader_ref = js_reader.reader.borrow_mut();
        let reader = match reader_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get reader").into(),
                ));
            }
        };
        let offset = number_arg(args, 0)?;
        if offset < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot seek negative bytes!").into(),
            ));
        }
        let size = number_arg(args, 1)?;
        if size < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot read negative bytes!").into(),
            ));
        }

        let bytes = match read_bytes(offset as u64, size as u64, None, reader) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read bytes via API: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::run::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::structs::toml::Output;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }
    #[test]
    fn test_js_buf_reader() {
        let test = "Ly8gLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBuYW1lOwogIG1lc3NhZ2U7CiAgY29uc3RydWN0b3IobmFtZSwgbWVzc2FnZSkgewogICAgc3VwZXIoKTsKICAgIHRoaXMubmFtZSA9IG5hbWU7CiAgICB0aGlzLm1lc3NhZ2UgPSBtZXNzYWdlOwogIH0KfTsKCi8vIC4uL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vcmVhZGVyLnRzCnZhciBCdWZSZWFkZXIgPSBjbGFzcyB7CiAgcmVhZGVyOwogIGNvbnN0cnVjdG9yKHBhdGgpIHsKICAgIHRoaXMucmVhZGVyID0gbmV3IEpzQnVmUmVhZGVyKHBhdGgpOwogIH0KICByZWFkQnl0ZXMob2Zmc2V0LCBieXRlcykgewogICAgaWYgKG9mZnNldCA8IDApIHsKICAgICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoYFJFQURFUmAsIGBDYW5ub3Qgc2VlayB0byBuZWdhdGl2ZSBvZmZzZXQgJHtvZmZzZXR9YCk7CiAgICB9CiAgICBpZiAoYnl0ZXMgPCAwKSB7CiAgICAgIHJldHVybiBuZXcgRmlsZUVycm9yKGBSRUFERVJgLCBgQ2Fubm90IHJlYWQgdG8gbmVnYXRpdmUgYnl0ZXMgJHtieXRlc31gKTsKICAgIH0KICAgIHRyeSB7CiAgICAgIGNvbnN0IHJlc3VsdHMgPSB0aGlzLnJlYWRlci5yZWFkKG9mZnNldCwgYnl0ZXMpOwogICAgICByZXR1cm4gcmVzdWx0czsKICAgIH0gY2F0Y2ggKGVycikgewogICAgICByZXR1cm4gbmV3IEZpbGVFcnJvcihgUkVBREVSYCwgYGNvdWxkIG5vdCByZWFkIGJ5dGVzICR7ZXJyfWApOwogICAgfQogIH0KfTsKCi8vIC4uLy4uL0Rvd25sb2Fkcy9tYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcmVhZGVyID0gbmV3IEJ1ZlJlYWRlcigiQzpcXFdpbmRvd3NcXGV4cGxvcmVyLmV4ZSIpOwogIGNvbnN0IGJ5dGVzID0gcmVhZGVyLnJlYWRCeXRlcygwLCAyNSk7CiAgaWYgKGJ5dGVzIGluc3RhbmNlb2YgRmlsZUVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGNvbnN0IGFycmF5ID0gQXJyYXkuZnJvbShieXRlcyk7CiAgaWYgKGFycmF5Lmxlbmd0aCAhPT0gMjUpIHsKICAgIHRocm93ICJiYWQgbGVuZ3RoIjsKICB9CiAgY29uc29sZS5sb2coYEkgdXNlZCB0aGUgUnVzdCBCdWZSZWFkZXIgdG8gcmVhZCB0aGUgZmlyc3QgMjUgYnl0ZXMgb2YgZXhwbG9yZXIuZXhlISAke2FycmF5fWApOwogIGNvbnN0IG1pZGRsZSA9IHJlYWRlci5yZWFkQnl0ZXMoMWUzLCA1MCk7CiAgaWYgKG1pZGRsZSBpbnN0YW5jZW9mIEZpbGVFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBjb25zb2xlLmxvZyhgSSB1c2VkIHRoZSBSdXN0IEJ1ZlJlYWRlciB0byByZWFkIDUwIGJ5dGVzIG9mIGV4cGxvcmVyLmV4ZSBhdCBvZmZzZXQgMTAwMCEgSSBkaWQgbm90IHJlYWQgdGhlIGVudGlyZSBmaWxlIGludG8gbWVtb3J5ISAke2FycmF5fWApOwogIHJldHVybiBBcnJheS5mcm9tKGJ5dGVzKTsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("js_reader"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

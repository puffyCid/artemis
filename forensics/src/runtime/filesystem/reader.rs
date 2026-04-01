use crate::{
    filesystem::ntfs::{
        raw_files::raw_reader,
        reader::read_bytes,
        sector_reader::SectorReader,
        setup::{NtfsParser, setup_ntfs_parser},
    },
    runtime::helper::{boolean_arg, number_arg, string_arg},
};
use boa_engine::{
    Context, JsData, JsError, JsResult, JsValue, NativeFunction,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsUint8Array,
};
use boa_gc::{Finalize, Trace};
use ntfs::{Ntfs, NtfsFile};
use std::{cell::RefCell, fs::File, io::BufReader, sync::Arc};

#[derive(Trace, Finalize, JsData)]
pub(crate) struct JsBufReader {
    /// Basically tells BoaJS garabage collector not to touch our BufReader.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    reader: RefCell<Option<BufReader<File>>>,
    #[unsafe_ignore_trace]
    ntfs: RefCell<Option<BufReader<SectorReader<File>>>>,
    #[unsafe_ignore_trace]
    ntfs_file: RefCell<Option<NtfsFile<'static>>>,
}

#[derive(Trace, Finalize, JsData)]
pub(crate) struct JsNtfsReader {
    /// Basically tells BoaJS garabage collector not to touch our BufReader.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    reader: Option<RefCell<Option<NtfsParser>>>,
}

/// Create a simple BufReader class that can be used to stream files
impl Class for JsBufReader {
    const NAME: &'static str = "JsBufReader";
    const LENGTH: usize = 2;

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(
            js_string!("read"),
            2,
            NativeFunction::from_fn_ptr(Self::read),
        );
        Ok(())
    }

    fn data_constructor(
        _this: &JsValue,
        args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        let path = string_arg(args, 0)?;
        let ntfs = boolean_arg(args, 1)?;
        println!("{path}");
        let file = match File::open(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not open {path}: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let reader = if !ntfs {
            JsBufReader {
                reader: RefCell::new(Some(BufReader::new(file))),
                ntfs: RefCell::new(None),
                ntfs_file: RefCell::new(None),
            }
        } else {
            let mut ntfs_parser = match setup_ntfs_parser(path.chars().next().unwrap_or('C')) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Could not setup ntfs parser {path}: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };
            let ntfs_leak: &'static Ntfs = Box::leak(Box::new(ntfs_parser.ntfs));
            let ntfs_file = match raw_reader(&path, &ntfs_leak, &mut ntfs_parser.fs) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Could not setup ntfs reader {path}: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };
            JsBufReader {
                reader: RefCell::new(Some(BufReader::new(file))),
                ntfs: RefCell::new(Some(ntfs_parser.fs)),
                ntfs_file: RefCell::new(Some(ntfs_file)),
            }
        };

        println!("done!");
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
                let issue = format!("Could read bytes via API: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }
}
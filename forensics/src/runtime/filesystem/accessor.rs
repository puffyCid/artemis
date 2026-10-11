use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{DirHandle, FileHandle},
        io::reader::AccessorReader,
        source::handle::SourceHandle,
    },
    runtime::helper::{number_arg, string_arg, value_arg},
};
use boa_engine::{
    Context, JsData, JsError, JsObject, JsResult, JsValue, NativeFunction,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsUint8Array,
};
use boa_gc::{Finalize, Trace};
use serde_json::Value;
use std::{cell::RefCell, io::Read};
use tracing::error;

/// Register the `Accessor` as a JavaScript class
#[derive(Trace, Finalize, JsData)]
pub(super) struct JsAccessor {
    /// An exposed `Accessor` to the `BoaJS` run time
    ///
    /// `unsafe_ignore_trace` is used to tell the `BoaJS` garbage
    /// collector not to touch our `Accessor`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    accessor: RefCell<Option<Accessor>>,
}

/// Expose the `Accessor` as a JavaScript class that can be used to interact with filesystem
impl Class for JsAccessor {
    const NAME: &'static str = "JsAccessor";
    const LENGTH: usize = 0;

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(
            js_string!("read_file"),
            1,
            NativeFunction::from_fn_ptr(Self::read_file),
        );

        class.method(
            js_string!("read_file_handle"),
            1,
            NativeFunction::from_fn_ptr(Self::read_file_handle),
        );

        class.method(
            js_string!("read_dir"),
            1,
            NativeFunction::from_fn_ptr(Self::read_dir),
        );

        class.method(
            js_string!("read_dir_handle"),
            1,
            NativeFunction::from_fn_ptr(Self::read_dir_handle),
        );

        class.method(
            js_string!("stat"),
            1,
            NativeFunction::from_fn_ptr(Self::stat),
        );

        class.method(
            js_string!("stat_handle"),
            1,
            NativeFunction::from_fn_ptr(Self::stat_handle),
        );

        class.method(
            js_string!("stat_dir_handle"),
            1,
            NativeFunction::from_fn_ptr(Self::stat_dir_handle),
        );

        class.method(
            js_string!("globfs"),
            1,
            NativeFunction::from_fn_ptr(Self::globfs),
        );

        class.method(
            js_string!("open_reader"),
            1,
            NativeFunction::from_fn_ptr(Self::open_reader),
        );

        class.method(
            js_string!("open_reader_handle"),
            1,
            NativeFunction::from_fn_ptr(Self::open_reader_handle),
        );

        class.method(
            js_string!("open_source"),
            1,
            NativeFunction::from_fn_ptr(Self::open_source),
        );

        class.method(
            js_string!("source_read_file"),
            2,
            NativeFunction::from_fn_ptr(Self::source_read_file),
        );

        class.method(
            js_string!("source_read_file_handle"),
            2,
            NativeFunction::from_fn_ptr(Self::source_read_file_handle),
        );

        class.method(
            js_string!("source_read_dir"),
            2,
            NativeFunction::from_fn_ptr(Self::source_read_dir),
        );

        class.method(
            js_string!("source_read_dir_handle"),
            2,
            NativeFunction::from_fn_ptr(Self::source_read_dir_handle),
        );

        class.method(
            js_string!("source_stat"),
            2,
            NativeFunction::from_fn_ptr(Self::source_stat),
        );

        class.method(
            js_string!("source_stat_handle"),
            2,
            NativeFunction::from_fn_ptr(Self::source_stat_handle),
        );

        class.method(
            js_string!("source_stat_dir_handle"),
            2,
            NativeFunction::from_fn_ptr(Self::source_stat_dir_handle),
        );

        class.method(
            js_string!("source_globfs"),
            2,
            NativeFunction::from_fn_ptr(Self::source_globfs),
        );

        class.method(
            js_string!("source_open_reader"),
            2,
            NativeFunction::from_fn_ptr(Self::source_open_reader),
        );

        class.method(
            js_string!("source_open_reader_handle"),
            2,
            NativeFunction::from_fn_ptr(Self::source_open_reader_handle),
        );

        Ok(())
    }

    /// Initial the structure of the `JsAccessor` class
    /// This is the `constructor` method when coding in JavaScript
    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        let accessor = Accessor::with_defaults();

        let js_accessor = JsAccessor {
            accessor: RefCell::new(Some(accessor)),
        };

        Ok(js_accessor)
    }
}

impl JsAccessor {
    /// Support reading files with the `Accessor` from JavaScript
    fn read_file(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // File to read
        let path = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let bytes = match accessor.read_file(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read bytes with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }

    /// Support reading `FileHandle` with the `Accessor` from JavaScript
    fn read_file_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let handle = Self::return_file_handle_object(value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let bytes = match accessor.read_file_handle(&handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read FileHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }

    /// Support reading directory with the `Accessor` from JavaScript
    fn read_dir(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // Directory to read
        let path = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.read_dir(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read directory with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support reading `DirHandle` with the `Accessor` from JavaScript
    fn read_dir_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let handle = Self::return_dir_handle_object(value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.read_dir_handle(&handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read DirHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat with the `Accessor` from JavaScript
    fn stat(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // File to read
        let path = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let meta = match accessor.stat(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat file: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&meta).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat `FileHandle` with the `Accessor` from JavaScript
    fn stat_handle(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let handle = Self::return_file_handle_object(value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let meta = match accessor.stat_handle(&handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat FileHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&meta).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat `DirHandle` with the `Accessor` from JavaScript
    fn stat_dir_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let handle = Self::return_dir_handle_object(value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.stat_dir_handle(&handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat DirHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support reading files with the `Accessor` from JavaScript
    fn globfs(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // File to read
        let path = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let matches = match accessor.globfs(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not glob with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&matches).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support returning a `AccessorReader` `Accessor` from JavaScript
    fn open_reader(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // File to read
        let path = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let reader = match accessor.open_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not create reader with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let js_accessor_reader = JsAccessorReader {
            reader: RefCell::new(Some(reader)),
        };

        let reader_obj = JsAccessorReader::from_data(js_accessor_reader, context)?;

        Ok(reader_obj.into())
    }

    /// Support returning a `AccessorReader` from a `FileHandle` with the `Accessor` from JavaScript
    fn open_reader_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let handle = Self::return_file_handle_object(value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let reader = match accessor.open_reader_handle(&handle) {
            Ok(result) => result,
            Err(err) => {
                let issue =
                    format!("Could not create reader from FileHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let js_accessor_reader = JsAccessorReader {
            reader: RefCell::new(Some(reader)),
        };

        let reader_obj = JsAccessorReader::from_data(js_accessor_reader, context)?;

        Ok(reader_obj.into())
    }

    /// Support opening a accessor source
    fn open_source(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // Source to open
        let source = string_arg(args, 0)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let handle = match accessor.open_source(&source) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read bytes with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&handle).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support reading a file from an opened source
    fn source_read_file(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let path = string_arg(args, 1)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let bytes = match accessor.source_read_file(&source, &path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read bytes with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }

    /// Support reading a `FileHandle` from an opened source
    fn source_read_file_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let file_value = value_arg(args, 1, context)?;
        let handle = Self::return_file_handle_object(file_value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };
        let bytes = match accessor.source_read_file_handle(&source, &handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read FileHandle with accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }

    /// Support reading a directory from an opened source
    fn source_read_dir(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let path = string_arg(args, 1)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.source_read_dir(&source, &path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read directory with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support reading a `DirHandle` from an opened source
    fn source_read_dir_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let dir_value = value_arg(args, 1, context)?;
        let handle = Self::return_dir_handle_object(dir_value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.source_read_dir_handle(&source, &handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not read DirHandle with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat a file from an opened source
    fn source_stat(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let path = string_arg(args, 1)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let meta = match accessor.source_stat(&source, &path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat path with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&meta).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat a `FileHandle` from an opened source
    fn source_stat_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let file_value = value_arg(args, 1, context)?;
        let handle = Self::return_file_handle_object(file_value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let meta = match accessor.source_stat_handle(&source, &handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat FileHandle with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&meta).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support stat a `DirHandle` from an opened source
    fn source_stat_dir_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let dir_value = value_arg(args, 1, context)?;
        let handle = Self::return_dir_handle_object(dir_value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let dir = match accessor.source_stat_dir_handle(&source, &handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not stat DirHandle with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&dir).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support globbing  from an opened source
    fn source_globfs(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let path = string_arg(args, 1)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let meta = match accessor.source_globfs(&source, &path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not glob path with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&meta).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        Ok(value)
    }

    /// Support opening a file from an opened source
    fn source_open_reader(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let path = string_arg(args, 1)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let reader = match accessor.source_open_reader(&source, &path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not create reader with source accessor: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let js_accessor_reader = JsAccessorReader {
            reader: RefCell::new(Some(reader)),
        };

        let reader_obj = JsAccessorReader::from_data(js_accessor_reader, context)?;

        Ok(reader_obj.into())
    }

    /// Support opening a `FileHandle` from an opened source
    fn source_open_reader_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let value = value_arg(args, 0, context)?;
        let source = Self::return_source_object(value)?;
        let file_value = value_arg(args, 1, context)?;
        let handle = Self::return_file_handle_object(file_value)?;
        let accessor_object = return_accessor_object(this, args, context)?;

        let js_accessor = match accessor_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessor Object").into(),
                ));
            }
        };

        let mut accessor_ref = js_accessor.accessor.borrow_mut();
        let accessor = match accessor_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor").into(),
                ));
            }
        };

        let reader = match accessor.source_open_reader_handle(&source, &handle) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!(
                    "Could not create reader from FileHandle with source accessor: {err:?}"
                );

                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let js_accessor_reader = JsAccessorReader {
            reader: RefCell::new(Some(reader)),
        };

        let reader_obj = JsAccessorReader::from_data(js_accessor_reader, context)?;

        Ok(reader_obj.into())
    }

    /// Deserialize the `SourceHandle`
    fn return_source_object(value: Value) -> JsResult<SourceHandle> {
        let handle: SourceHandle = match serde_json::from_value(value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize SourceHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        Ok(handle)
    }

    /// Deserialize the `FileHandle`
    fn return_file_handle_object(value: Value) -> JsResult<FileHandle> {
        let handle: FileHandle = match serde_json::from_value(value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize FileHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        Ok(handle)
    }

    /// Deserialize the `DirHandle`
    fn return_dir_handle_object(value: Value) -> JsResult<DirHandle> {
        let handle: DirHandle = match serde_json::from_value(value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize DirHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        Ok(handle)
    }
}

/// Register the `AccessorReader` as a JavaScript class
#[derive(Trace, Finalize, JsData)]
pub(super) struct JsAccessorReader {
    /// An exposed `AccessorReader` to the `BoaJS` run time
    ///
    /// `unsafe_ignore_trace` is used to tell the `BoaJS` garbage
    /// collector not to touch our `AccessorReader`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    reader: RefCell<Option<AccessorReader>>,
}

impl Class for JsAccessorReader {
    const NAME: &'static str = "JsAccessorReader";
    //const LENGTH: usize = 1;

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(
            js_string!("read_at"),
            2,
            NativeFunction::from_fn_ptr(Self::read_at),
        );

        class.method(
            js_string!("read"),
            1,
            NativeFunction::from_fn_ptr(Self::read),
        );

        class.method(
            js_string!("seek"),
            1,
            NativeFunction::from_fn_ptr(Self::seek),
        );

        Ok(())
    }

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        let issue = "You cannot construct an AccessorReader. Use Accessor.open_reader or Accessor.open_reader_handle";
        Err(JsError::from_opaque(js_string!(issue).into()))
    }
}

impl JsAccessorReader {
    /// Read bytes from a file at the provided offset (from start)
    fn read_at(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let offset = number_arg(args, 0)?;
        if offset < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot seek negative bytes!").into(),
            ));
        }

        let length = number_arg(args, 1)?;
        if length < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot read negative bytes!").into(),
            ));
        }

        let reader_object = return_accessor_object(this, args, context)?;
        let js_reader = match reader_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessorReader Object").into(),
                ));
            }
        };

        let mut reader_ref = js_reader.reader.borrow_mut();
        let reader = match reader_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor reader").into(),
                ));
            }
        };

        let bytes = match reader.read_bytes(offset as u64, length as usize) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Could not read bytes with reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let value = JsUint8Array::from_iter(bytes, context)?;

        Ok(value.into())
    }

    /// Read bytes from a file at current offset
    fn read(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let length = number_arg(args, 0)?;
        if length < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot read negative bytes!").into(),
            ));
        }

        let reader_object = return_accessor_object(this, args, context)?;
        let js_reader = match reader_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessorReader Object").into(),
                ));
            }
        };

        let mut reader_ref = js_reader.reader.borrow_mut();
        let reader = match reader_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor reader").into(),
                ));
            }
        };

        let mut buf = vec![0; length as usize];
        if let Err(err) = reader.read_exact(&mut buf) {
            let issue = format!("Could not read bytes with reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }

        let value = JsUint8Array::from_iter(buf, context)?;

        Ok(value.into())
    }

    /// Seek to offset of a file (from start)
    fn seek(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let offset = number_arg(args, 0)?;
        if offset < 0.0 {
            return Err(JsError::from_opaque(
                js_string!("Cannot seek negative bytes!").into(),
            ));
        }

        let reader_object = return_accessor_object(this, args, context)?;
        let js_reader = match reader_object.downcast_mut::<Self>() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Not a JsAccessorReader Object").into(),
                ));
            }
        };

        let mut reader_ref = js_reader.reader.borrow_mut();
        let reader = match reader_ref.as_mut() {
            Some(result) => result,
            None => {
                return Err(JsError::from_opaque(
                    js_string!("Could not get accessor reader").into(),
                ));
            }
        };

        let position = match reader.seek_from_start(offset as u64) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Could not seek {offset} with reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        Ok(position.into())
    }
}

/// Return the provided `JsObject`
fn return_accessor_object(
    this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsObject> {
    let obj_accessor = match this.as_object() {
        Some(result) => result,
        None => {
            return Err(JsError::from_opaque(js_string!("Not an Object").into()));
        }
    };

    Ok(obj_accessor)
}

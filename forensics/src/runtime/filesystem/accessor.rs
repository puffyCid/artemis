use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{DirHandle, FileHandle},
        io::reader::AccessorReader,
    },
    runtime::helper::{string_arg, value_arg},
};
use boa_engine::{
    Context, JsData, JsError, JsObject, JsResult, JsValue, NativeFunction,
    class::{Class, ClassBuilder},
    js_string,
    object::builtins::JsUint8Array,
};
use boa_gc::{Finalize, Trace};
use std::cell::RefCell;
use tracing::error;

#[derive(Trace, Finalize, JsData)]
pub(super) struct JsAccessor {
    /// An exposed `Accessor` to the BoaJS run time
    ///
    /// `unsafe_ignore_trace` is used to tell the `BoaJS` garbage
    /// collector not to touch our `Accessor`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    accessor: RefCell<Option<Accessor>>,
}

#[derive(Trace, Finalize, JsData)]
pub(super) struct JsAccessorReader {
    /// An exposed `Accessor` to the BoaJS run time
    ///
    /// `unsafe_ignore_trace` is used to tell the `BoaJS` garbage
    /// collector not to touch our `Accessor`.
    /// The garbage collector cannot trace this
    #[unsafe_ignore_trace]
    accessor: RefCell<Option<AccessorReader>>,
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
        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let file_value = value_arg(args, 0, context)?;
        let handle: FileHandle = match serde_json::from_value(file_value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize FileHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let dir_value = value_arg(args, 0, context)?;
        let handle: DirHandle = match serde_json::from_value(dir_value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize DirHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let file_value = value_arg(args, 0, context)?;
        let handle: FileHandle = match serde_json::from_value(file_value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize FileHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let dir_value = value_arg(args, 0, context)?;
        let handle: DirHandle = match serde_json::from_value(dir_value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize DirHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
            accessor: RefCell::new(Some(reader)),
        };

        let proto = context.intrinsics().constructors().object().prototype();
        let reader_obj = JsObject::from_proto_and_data(proto, js_accessor_reader);

        Ok(reader_obj.into())
    }

    /// Support returning a `AccessorReader` from a `FileHandle` with the `Accessor` from JavaScript
    fn open_reader_handle(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let file_value = value_arg(args, 0, context)?;
        let handle: FileHandle = match serde_json::from_value(file_value) {
            Ok(results) => results,
            Err(err) => {
                let issue = format!("Failed to deserialize FileHandle format: {err:?}");

                error!(issue);
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let accessor_object = Self::return_accessor_object(this, args, context)?;

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
            accessor: RefCell::new(Some(reader)),
        };

        let proto = context.intrinsics().constructors().object().prototype();
        let reader_obj = JsObject::from_proto_and_data(proto, js_accessor_reader);

        Ok(reader_obj.into())
    }

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
}

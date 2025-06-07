use crate::{
    artifacts::os::windows::ese::{
        helper::{
            dump_table_columns, get_all_pages, get_catalog_info, get_filtered_page_data,
            get_page_data,
        },
        tables::TableInfo,
    },
    runtime::helper::{number_arg, string_arg, value_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use std::collections::HashMap;

pub(crate) fn js_get_catalog(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let ese = match get_catalog_info(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get catalog for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&ese).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_get_pages(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let first_page = number_arg(args, &1)? as u32;
    let ese = match get_all_pages(&path, &first_page) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get pages for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&ese).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_page_data(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let pages_value = value_arg(args, &1, context)?;
    let pages: Vec<u32> = match serde_json::from_value(pages_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize pages: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let info_value = value_arg(args, &2, context)?;
    let mut info: TableInfo = match serde_json::from_value(info_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let name = string_arg(args, &3)?;

    let ese = match get_page_data(&path, &pages, &mut info, &name) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get page data for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&ese).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_filter_page_data(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let pages_value = value_arg(args, &1, context)?;
    let pages: Vec<u32> = match serde_json::from_value(pages_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize pages: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let info_value = value_arg(args, &2, context)?;
    let mut info: TableInfo = match serde_json::from_value(info_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let name = string_arg(args, &3)?;
    let column_name = string_arg(args, &4)?;

    let columns_serde = value_arg(args, &5, context)?;
    let mut column_values: HashMap<String, bool> = match serde_json::from_value(columns_serde) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let ese = match get_filtered_page_data(
        &path,
        &pages,
        &mut info,
        &name,
        &column_name,
        &mut column_values,
    ) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get filtered page data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&ese).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_get_table_columns(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let pages_value = value_arg(args, &1, context)?;
    let pages: Vec<u32> = match serde_json::from_value(pages_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize pages: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let info_value = value_arg(args, &2, context)?;
    let mut info: TableInfo = match serde_json::from_value(info_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let name = string_arg(args, &3)?;

    let columns_value: serde_json::Value = value_arg(args, &4, context)?;
    let column_names: Vec<String> = match serde_json::from_value(columns_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let ese = match dump_table_columns(&path, &pages, &mut info, &name, &column_names) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to dump table columns for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&ese).unwrap_or_default();
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
            format: String::from("json"),
            compress,
            timeline: false,
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
    fn test_js_get_catalog() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9lc2UudHMKZnVuY3Rpb24gY2F0YWxvZ0luZm8ocGF0aCkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfZ2V0X2NhdGFsb2cocGF0aCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJFU0UiLCBgZmFpbGVkIHRvIHBhcnNlIGVzZSAke3BhdGh9OiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCByZXN1bHRzID0gY2F0YWxvZ0luZm8oIkM6XFxQcm9ncmFtRGF0YVxcTWljcm9zb2Z0XFxTZWFyY2hcXERhdGFcXEFwcGxpY2F0aW9uc1xcV2luZG93c1xcV2luZG93cy5lZGIiKTsKICBpZiAocmVzdWx0cyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IGVudHJ5IG9mIHJlc3VsdHMpIHsKICAgIGNvbnNvbGUubG9nKGBOYW1lOiAke2VudHJ5Lm5hbWV9YCk7CiAgICBicmVhazsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("ese_catalog"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_get_pages_and_data_rows() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9lc2UudHMKZnVuY3Rpb24gY2F0YWxvZ0luZm8ocGF0aCkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfZ2V0X2NhdGFsb2cocGF0aCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKAogICAgICAiRVNFIiwKICAgICAgYGZhaWxlZCB0byBwYXJzZSBlc2UgY2F0YWxvZyAke3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQpmdW5jdGlvbiB0YWJsZUluZm8oY2F0YWxvZywgdGFibGVfbmFtZSkgewogIGNvbnN0IGluZm8gPSB7CiAgICBvYmpfaWRfdGFibGU6IDAsCiAgICB0YWJsZV9wYWdlOiAwLAogICAgdGFibGVfbmFtZTogIiIsCiAgICBjb2x1bW5faW5mbzogW10sCiAgICBsb25nX3ZhbHVlX3BhZ2U6IDAKICB9OwogIGZvciAoY29uc3QgZW50cnkgb2YgY2F0YWxvZykgewogICAgaWYgKGVudHJ5Lm5hbWUgPT09IHRhYmxlX25hbWUpIHsKICAgICAgaW5mby50YWJsZV9uYW1lID0gZW50cnkubmFtZTsKICAgICAgaW5mby5vYmpfaWRfdGFibGUgPSBlbnRyeS5vYmpfaWRfdGFibGU7CiAgICAgIGluZm8udGFibGVfcGFnZSA9IGVudHJ5LmNvbHVtbl9vcl9mYXRoZXJfZGF0YV9wYWdlOwogICAgICBjb250aW51ZTsKICAgIH0KICAgIGlmIChlbnRyeS5vYmpfaWRfdGFibGUgPT09IGluZm8ub2JqX2lkX3RhYmxlICYmIGluZm8udGFibGVfbmFtZS5sZW5ndGggIT0gMCAmJiBlbnRyeS5jYXRhbG9nX3R5cGUgPT09ICJDb2x1bW4iIC8qIENvbHVtbiAqLykgewogICAgICBjb25zdCBjb2x1bW5faW5mbyA9IHsKICAgICAgICBjb2x1bW5fdHlwZTogZ2V0Q29sdW1uVHlwZShlbnRyeS5jb2x1bW5fb3JfZmF0aGVyX2RhdGFfcGFnZSksCiAgICAgICAgY29sdW1uX25hbWU6IGVudHJ5Lm5hbWUsCiAgICAgICAgY29sdW1uX2RhdGE6IFtdLAogICAgICAgIGNvbHVtbl9pZDogZW50cnkuaWQsCiAgICAgICAgY29sdW1uX2ZsYWdzOiBnZXRDb2x1bW5GbGFncyhlbnRyeS5mbGFncyksCiAgICAgICAgY29sdW1uX3NwYWNlX3VzYWdlOiBlbnRyeS5zcGFjZV91c2FnZSwKICAgICAgICBjb2x1bW5fdGFnZ2VkX2ZsYWdzOiBbXQogICAgICB9OwogICAgICBpbmZvLmNvbHVtbl9pbmZvLnB1c2goY29sdW1uX2luZm8pOwogICAgfSBlbHNlIGlmIChlbnRyeS5vYmpfaWRfdGFibGUgPT09IGluZm8ub2JqX2lkX3RhYmxlICYmIGluZm8udGFibGVfbmFtZS5sZW5ndGggIT0gMCAmJiBlbnRyeS5jYXRhbG9nX3R5cGUgPT09ICJMb25nVmFsdWUiIC8qIExvbmdWYWx1ZSAqLykgewogICAgICBpbmZvLmxvbmdfdmFsdWVfcGFnZSA9IGVudHJ5LmNvbHVtbl9vcl9mYXRoZXJfZGF0YV9wYWdlOwogICAgfQogIH0KICByZXR1cm4gaW5mbzsKfQpmdW5jdGlvbiBnZXRQYWdlcyhwYXRoLCBmaXJzdF9wYWdlKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfcGFnZXMocGF0aCwgZmlyc3RfcGFnZSk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJFU0UiLCBgZmFpbGVkIHRvIHBhcnNlIGVzZSBwYWdlcyAke3BhdGh9OiAke2Vycn1gKTsKICB9Cn0KZnVuY3Rpb24gZ2V0Um93cyhwYXRoLCBwYWdlcywgaW5mbywgbmFtZSkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfcGFnZV9kYXRhKHBhdGgsIHBhZ2VzLCBpbmZvLCBuYW1lKTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIkVTRSIsIGBmYWlsZWQgdG8gcGFyc2UgZXNlIHJvd3MgJHtwYXRofTogJHtlcnJ9YCk7CiAgfQp9CmZ1bmN0aW9uIGdldENvbHVtblR5cGUoY29sdW1uX3R5cGUpIHsKICBzd2l0Y2ggKGNvbHVtbl90eXBlKSB7CiAgICBjYXNlIDA6CiAgICAgIHJldHVybiAiTmlsIiAvKiBOaWwgKi87CiAgICBjYXNlIDE6CiAgICAgIHJldHVybiAiQml0IiAvKiBCaXQgKi87CiAgICBjYXNlIDI6CiAgICAgIHJldHVybiAiVW5zaWduZWRCeXRlIiAvKiBVbnNpZ25lZEJ5dGUgKi87CiAgICBjYXNlIDM6CiAgICAgIHJldHVybiAiU2hvcnQiIC8qIFNob3J0ICovOwogICAgY2FzZSA0OgogICAgICByZXR1cm4gIkxvbmciIC8qIExvbmcgKi87CiAgICBjYXNlIDU6CiAgICAgIHJldHVybiAiQ3VycmVuY3kiIC8qIEN1cnJlbmN5ICovOwogICAgY2FzZSA2OgogICAgICByZXR1cm4gIkZsb2F0MzIiIC8qIEZsb2F0MzIgKi87CiAgICBjYXNlIDc6CiAgICAgIHJldHVybiAiRmxvYXQ2NCIgLyogRmxvYXQ2NCAqLzsKICAgIGNhc2UgODoKICAgICAgcmV0dXJuICJEYXRlVGltZSIgLyogRGF0ZVRpbWUgKi87CiAgICBjYXNlIDk6CiAgICAgIHJldHVybiAiQmluYXJ5IiAvKiBCaW5hcnkgKi87CiAgICBjYXNlIDEwOgogICAgICByZXR1cm4gIlRleHQiIC8qIFRleHQgKi87CiAgICBjYXNlIDExOgogICAgICByZXR1cm4gIkxvbmdCaW5hcnkiIC8qIExvbmdCaW5hcnkgKi87CiAgICBjYXNlIDEyOgogICAgICByZXR1cm4gIkxvbmdUZXh0IiAvKiBMb25nVGV4dCAqLzsKICAgIGNhc2UgMTM6CiAgICAgIHJldHVybiAiU3VwZXJMb25nIiAvKiBTdXBlckxvbmcgKi87CiAgICBjYXNlIDE0OgogICAgICByZXR1cm4gIlVuc2lnbmVkTG9uZyIgLyogVW5zaWduZWRMb25nICovOwogICAgY2FzZSAxNToKICAgICAgcmV0dXJuICJMb25nTG9uZyIgLyogTG9uZ0xvbmcgKi87CiAgICBjYXNlIDE2OgogICAgICByZXR1cm4gIkd1aWQiIC8qIEd1aWQgKi87CiAgICBjYXNlIDE3OgogICAgICByZXR1cm4gIlVuc2lnbmVkU2hvcnQiIC8qIFVuc2lnbmVkU2hvcnQgKi87CiAgICBkZWZhdWx0OgogICAgICByZXR1cm4gIlVua25vd24iIC8qIFVua25vd24gKi87CiAgfQp9CmZ1bmN0aW9uIGdldENvbHVtbkZsYWdzKGZsYWdzKSB7CiAgY29uc3Qgbm90X251bGwgPSAxOwogIGNvbnN0IHZlcnNpb24gPSAyOwogIGNvbnN0IGluY3JlbWVudCA9IDQ7CiAgY29uc3QgbXVsdGkgPSA4OwogIGNvbnN0IGZsYWdfZGVmYXVsdCA9IDE2OwogIGNvbnN0IGVzY3JvdyA9IDMyOwogIGNvbnN0IGZpbmFsaXplID0gNjQ7CiAgY29uc3QgdXNlcl9kZWZpbmUgPSAxMjg7CiAgY29uc3QgdGVtcGxhdGUgPSAyNTY7CiAgY29uc3QgZGVsZXRlX3plcm8gPSA1MTI7CiAgY29uc3QgcHJpbWFyeSA9IDIwNDg7CiAgY29uc3QgY29tcHJlc3NlZCA9IDQwOTY7CiAgY29uc3QgZW5jcnlwdGVkID0gODE5MjsKICBjb25zdCB2ZXJzaW9uZWQgPSA2NTUzNjsKICBjb25zdCBkZWxldGVkID0gMTMxMDcyOwogIGNvbnN0IHZlcnNpb25fYWRkID0gMjYyMTQ0OwogIGNvbnN0IGZsYWdzX2RhdGEgPSBbXTsKICBpZiAoKGZsYWdzICYgbm90X251bGwpID09PSBub3RfbnVsbCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJOb3ROdWxsIiAvKiBOb3ROdWxsICovKTsKICB9CiAgaWYgKChmbGFncyAmIHZlcnNpb24pID09PSB2ZXJzaW9uKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIlZlcnNpb24iIC8qIFZlcnNpb24gKi8pOwogIH0KICBpZiAoKGZsYWdzICYgaW5jcmVtZW50KSA9PT0gaW5jcmVtZW50KSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkF1dG9JbmNyZW1lbnQiIC8qIEF1dG9JbmNyZW1lbnQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgbXVsdGkpID09PSBtdWx0aSkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJNdWx0aVZhbHVlZCIgLyogTXVsdGlWYWx1ZWQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgZmxhZ19kZWZhdWx0KSA9PT0gZmxhZ19kZWZhdWx0KSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkRlZmF1bHQiIC8qIERlZmF1bHQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgZXNjcm93KSA9PT0gZXNjcm93KSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkVzY3Jvd1VwZGF0ZSIgLyogRXNjcm93VXBkYXRlICovKTsKICB9CiAgaWYgKChmbGFncyAmIGZpbmFsaXplKSA9PT0gZmluYWxpemUpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiRmluYWxpemUiIC8qIEZpbmFsaXplICovKTsKICB9CiAgaWYgKChmbGFncyAmIHVzZXJfZGVmaW5lKSA9PT0gdXNlcl9kZWZpbmUpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiVXNlckRlZmluZWREZWZhdWx0IiAvKiBVc2VyRGVmaW5lZERlZmF1bHQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgdGVtcGxhdGUpID09PSB0ZW1wbGF0ZSkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJUZW1wbGF0ZUNvbHVtbkVTRTk4IiAvKiBUZW1wbGF0ZUNvbHVtbkVTRTk4ICovKTsKICB9CiAgaWYgKChmbGFncyAmIGRlbGV0ZV96ZXJvKSA9PT0gZGVsZXRlX3plcm8pIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiRGVsZXRlT25aZXJvIiAvKiBEZWxldGVPblplcm8gKi8pOwogIH0KICBpZiAoKGZsYWdzICYgcHJpbWFyeSkgPT09IHByaW1hcnkpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiUHJpbWFyeUluZGV4UGxhY2Vob2xkZXIiIC8qIFByaW1hcnlJbmRleFBsYWNlaG9sZGVyICovKTsKICB9CiAgaWYgKChmbGFncyAmIGNvbXByZXNzZWQpID09PSBjb21wcmVzc2VkKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkNvbXByZXNzZWQiIC8qIENvbXByZXNzZWQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgZW5jcnlwdGVkKSA9PT0gZW5jcnlwdGVkKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkVuY3J5cHRlZCIgLyogRW5jcnlwdGVkICovKTsKICB9CiAgaWYgKChmbGFncyAmIHZlcnNpb25lZCkgPT09IHZlcnNpb25lZCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJWZXJzaW9uZWQiIC8qIFZlcnNpb25lZCAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiBkZWxldGVkKSA9PT0gZGVsZXRlZCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJEZWxldGVkIiAvKiBEZWxldGVkICovKTsKICB9CiAgaWYgKChmbGFncyAmIHZlcnNpb25fYWRkKSA9PT0gdmVyc2lvbl9hZGQpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiVmVyc2lvbmVkQWRkIiAvKiBWZXJzaW9uZWRBZGQgKi8pOwogIH0KICByZXR1cm4gZmxhZ3NfZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXHNlY3VyaXR5XFxkYXRhYmFzZVxcc2VjZWRpdC5zZGIiOwogIGNvbnN0IHRhYmxlID0gIlNtVGJsU21wIjsKICBjb25zdCByZXN1bHRzID0gY2F0YWxvZ0luZm8ocGF0aCk7CiAgaWYgKHJlc3VsdHMgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgY29uc3QgaW5mbyA9IHRhYmxlSW5mbyhyZXN1bHRzLCB0YWJsZSk7CiAgaWYgKGluZm8gaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgY29uc3QgcGFnZXMgPSBnZXRQYWdlcyhwYXRoLCBpbmZvLnRhYmxlX3BhZ2UpOwogIGlmIChwYWdlcyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBsZXQgc3VtID0gMDsKICBjb25zdCBwYWdlX2xpbWl0ID0gMTA7CiAgbGV0IHBhZ2VfY2h1bmsgPSBbXTsKICBmb3IgKGNvbnN0IHBhZ2Ugb2YgcGFnZXMpIHsKICAgIGlmIChwYWdlID09PSAwKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgcGFnZV9jaHVuay5wdXNoKHBhZ2UpOwogICAgaWYgKHBhZ2VfY2h1bmsubGVuZ3RoICE9IHBhZ2VfbGltaXQpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCByb3dzID0gZ2V0Um93cyhwYXRoLCBwYWdlX2NodW5rLCBpbmZvLCB0YWJsZSk7CiAgICBpZiAocm93cyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgewogICAgICByZXR1cm47CiAgICB9CiAgICBzdW0gKz0gcm93c1t0YWJsZV0ubGVuZ3RoOwogICAgcGFnZV9jaHVuayA9IFtdOwogIH0KICBpZiAocGFnZV9jaHVuay5sZW5ndGggIT0gMCkgewogICAgY29uc3Qgcm93cyA9IGdldFJvd3MocGF0aCwgcGFnZV9jaHVuaywgaW5mbywgdGFibGUpOwogICAgaWYgKHJvd3MgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgICAgcmV0dXJuOwogICAgfQogICAgc3VtICs9IHJvd3NbIlNtVGJsU21wIl0ubGVuZ3RoOwogIH0KICBjb25zb2xlLmxvZyhgVG90YWwgcm93czogJHtzdW19YCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("ese_rows"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_get_columns_and_filter() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9lc2UudHMKZnVuY3Rpb24gY2F0YWxvZ0luZm8ocGF0aCkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfZ2V0X2NhdGFsb2cocGF0aCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKAogICAgICAiRVNFIiwKICAgICAgYGZhaWxlZCB0byBwYXJzZSBlc2UgY2F0YWxvZyAke3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQpmdW5jdGlvbiB0YWJsZUluZm8oY2F0YWxvZywgdGFibGVfbmFtZSkgewogIGNvbnN0IGluZm8gPSB7CiAgICBvYmpfaWRfdGFibGU6IDAsCiAgICB0YWJsZV9wYWdlOiAwLAogICAgdGFibGVfbmFtZTogIiIsCiAgICBjb2x1bW5faW5mbzogW10sCiAgICBsb25nX3ZhbHVlX3BhZ2U6IDAKICB9OwogIGZvciAoY29uc3QgZW50cnkgb2YgY2F0YWxvZykgewogICAgaWYgKGVudHJ5Lm5hbWUgPT09IHRhYmxlX25hbWUpIHsKICAgICAgaW5mby50YWJsZV9uYW1lID0gZW50cnkubmFtZTsKICAgICAgaW5mby5vYmpfaWRfdGFibGUgPSBlbnRyeS5vYmpfaWRfdGFibGU7CiAgICAgIGluZm8udGFibGVfcGFnZSA9IGVudHJ5LmNvbHVtbl9vcl9mYXRoZXJfZGF0YV9wYWdlOwogICAgICBjb250aW51ZTsKICAgIH0KICAgIGlmIChlbnRyeS5vYmpfaWRfdGFibGUgPT09IGluZm8ub2JqX2lkX3RhYmxlICYmIGluZm8udGFibGVfbmFtZS5sZW5ndGggIT0gMCAmJiBlbnRyeS5jYXRhbG9nX3R5cGUgPT09ICJDb2x1bW4iIC8qIENvbHVtbiAqLykgewogICAgICBjb25zdCBjb2x1bW5faW5mbyA9IHsKICAgICAgICBjb2x1bW5fdHlwZTogZ2V0Q29sdW1uVHlwZShlbnRyeS5jb2x1bW5fb3JfZmF0aGVyX2RhdGFfcGFnZSksCiAgICAgICAgY29sdW1uX25hbWU6IGVudHJ5Lm5hbWUsCiAgICAgICAgY29sdW1uX2RhdGE6IFtdLAogICAgICAgIGNvbHVtbl9pZDogZW50cnkuaWQsCiAgICAgICAgY29sdW1uX2ZsYWdzOiBnZXRDb2x1bW5GbGFncyhlbnRyeS5mbGFncyksCiAgICAgICAgY29sdW1uX3NwYWNlX3VzYWdlOiBlbnRyeS5zcGFjZV91c2FnZSwKICAgICAgICBjb2x1bW5fdGFnZ2VkX2ZsYWdzOiBbXQogICAgICB9OwogICAgICBpbmZvLmNvbHVtbl9pbmZvLnB1c2goY29sdW1uX2luZm8pOwogICAgfSBlbHNlIGlmIChlbnRyeS5vYmpfaWRfdGFibGUgPT09IGluZm8ub2JqX2lkX3RhYmxlICYmIGluZm8udGFibGVfbmFtZS5sZW5ndGggIT0gMCAmJiBlbnRyeS5jYXRhbG9nX3R5cGUgPT09ICJMb25nVmFsdWUiIC8qIExvbmdWYWx1ZSAqLykgewogICAgICBpbmZvLmxvbmdfdmFsdWVfcGFnZSA9IGVudHJ5LmNvbHVtbl9vcl9mYXRoZXJfZGF0YV9wYWdlOwogICAgfQogIH0KICByZXR1cm4gaW5mbzsKfQpmdW5jdGlvbiBnZXRQYWdlcyhwYXRoLCBmaXJzdF9wYWdlKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfcGFnZXMocGF0aCwgZmlyc3RfcGFnZSk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJFU0UiLCBgZmFpbGVkIHRvIHBhcnNlIGVzZSBwYWdlcyAke3BhdGh9OiAke2Vycn1gKTsKICB9Cn0KZnVuY3Rpb24gZ2V0RmlsdGVyZWRSb3dzKHBhdGgsIHBhZ2VzLCBpbmZvLCBuYW1lLCBjb2x1bW5fbmFtZSwgY29sdW1uX2RhdGEpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IGpzX2ZpbHRlcl9wYWdlX2RhdGEocGF0aCwgcGFnZXMsIGluZm8sIG5hbWUsIGNvbHVtbl9uYW1lLCBjb2x1bW5fZGF0YSk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJFU0UiLCBgZmFpbGVkIHRvIHBhcnNlIGVzZSByb3dzICR7cGF0aH06ICR7ZXJyfWApOwogIH0KfQpmdW5jdGlvbiBkdW1wVGFibGVDb2x1bW5zKHBhdGgsIHBhZ2VzLCBpbmZvLCBuYW1lLCBjb2x1bW5fbmFtZXMpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3RhYmxlX2NvbHVtbnMocGF0aCwgcGFnZXMsIGluZm8sIG5hbWUsIGNvbHVtbl9uYW1lcyk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJFU0UiLCBgZmFpbGVkIHRvIHBhcnNlIGVzZSByb3dzICR7cGF0aH06ICR7ZXJyfWApOwogIH0KfQpmdW5jdGlvbiBnZXRDb2x1bW5UeXBlKGNvbHVtbl90eXBlKSB7CiAgc3dpdGNoIChjb2x1bW5fdHlwZSkgewogICAgY2FzZSAwOgogICAgICByZXR1cm4gIk5pbCIgLyogTmlsICovOwogICAgY2FzZSAxOgogICAgICByZXR1cm4gIkJpdCIgLyogQml0ICovOwogICAgY2FzZSAyOgogICAgICByZXR1cm4gIlVuc2lnbmVkQnl0ZSIgLyogVW5zaWduZWRCeXRlICovOwogICAgY2FzZSAzOgogICAgICByZXR1cm4gIlNob3J0IiAvKiBTaG9ydCAqLzsKICAgIGNhc2UgNDoKICAgICAgcmV0dXJuICJMb25nIiAvKiBMb25nICovOwogICAgY2FzZSA1OgogICAgICByZXR1cm4gIkN1cnJlbmN5IiAvKiBDdXJyZW5jeSAqLzsKICAgIGNhc2UgNjoKICAgICAgcmV0dXJuICJGbG9hdDMyIiAvKiBGbG9hdDMyICovOwogICAgY2FzZSA3OgogICAgICByZXR1cm4gIkZsb2F0NjQiIC8qIEZsb2F0NjQgKi87CiAgICBjYXNlIDg6CiAgICAgIHJldHVybiAiRGF0ZVRpbWUiIC8qIERhdGVUaW1lICovOwogICAgY2FzZSA5OgogICAgICByZXR1cm4gIkJpbmFyeSIgLyogQmluYXJ5ICovOwogICAgY2FzZSAxMDoKICAgICAgcmV0dXJuICJUZXh0IiAvKiBUZXh0ICovOwogICAgY2FzZSAxMToKICAgICAgcmV0dXJuICJMb25nQmluYXJ5IiAvKiBMb25nQmluYXJ5ICovOwogICAgY2FzZSAxMjoKICAgICAgcmV0dXJuICJMb25nVGV4dCIgLyogTG9uZ1RleHQgKi87CiAgICBjYXNlIDEzOgogICAgICByZXR1cm4gIlN1cGVyTG9uZyIgLyogU3VwZXJMb25nICovOwogICAgY2FzZSAxNDoKICAgICAgcmV0dXJuICJVbnNpZ25lZExvbmciIC8qIFVuc2lnbmVkTG9uZyAqLzsKICAgIGNhc2UgMTU6CiAgICAgIHJldHVybiAiTG9uZ0xvbmciIC8qIExvbmdMb25nICovOwogICAgY2FzZSAxNjoKICAgICAgcmV0dXJuICJHdWlkIiAvKiBHdWlkICovOwogICAgY2FzZSAxNzoKICAgICAgcmV0dXJuICJVbnNpZ25lZFNob3J0IiAvKiBVbnNpZ25lZFNob3J0ICovOwogICAgZGVmYXVsdDoKICAgICAgcmV0dXJuICJVbmtub3duIiAvKiBVbmtub3duICovOwogIH0KfQpmdW5jdGlvbiBnZXRDb2x1bW5GbGFncyhmbGFncykgewogIGNvbnN0IG5vdF9udWxsID0gMTsKICBjb25zdCB2ZXJzaW9uID0gMjsKICBjb25zdCBpbmNyZW1lbnQgPSA0OwogIGNvbnN0IG11bHRpID0gODsKICBjb25zdCBmbGFnX2RlZmF1bHQgPSAxNjsKICBjb25zdCBlc2Nyb3cgPSAzMjsKICBjb25zdCBmaW5hbGl6ZSA9IDY0OwogIGNvbnN0IHVzZXJfZGVmaW5lID0gMTI4OwogIGNvbnN0IHRlbXBsYXRlID0gMjU2OwogIGNvbnN0IGRlbGV0ZV96ZXJvID0gNTEyOwogIGNvbnN0IHByaW1hcnkgPSAyMDQ4OwogIGNvbnN0IGNvbXByZXNzZWQgPSA0MDk2OwogIGNvbnN0IGVuY3J5cHRlZCA9IDgxOTI7CiAgY29uc3QgdmVyc2lvbmVkID0gNjU1MzY7CiAgY29uc3QgZGVsZXRlZCA9IDEzMTA3MjsKICBjb25zdCB2ZXJzaW9uX2FkZCA9IDI2MjE0NDsKICBjb25zdCBmbGFnc19kYXRhID0gW107CiAgaWYgKChmbGFncyAmIG5vdF9udWxsKSA9PT0gbm90X251bGwpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiTm90TnVsbCIgLyogTm90TnVsbCAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiB2ZXJzaW9uKSA9PT0gdmVyc2lvbikgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJWZXJzaW9uIiAvKiBWZXJzaW9uICovKTsKICB9CiAgaWYgKChmbGFncyAmIGluY3JlbWVudCkgPT09IGluY3JlbWVudCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJBdXRvSW5jcmVtZW50IiAvKiBBdXRvSW5jcmVtZW50ICovKTsKICB9CiAgaWYgKChmbGFncyAmIG11bHRpKSA9PT0gbXVsdGkpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiTXVsdGlWYWx1ZWQiIC8qIE11bHRpVmFsdWVkICovKTsKICB9CiAgaWYgKChmbGFncyAmIGZsYWdfZGVmYXVsdCkgPT09IGZsYWdfZGVmYXVsdCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJEZWZhdWx0IiAvKiBEZWZhdWx0ICovKTsKICB9CiAgaWYgKChmbGFncyAmIGVzY3JvdykgPT09IGVzY3JvdykgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJFc2Nyb3dVcGRhdGUiIC8qIEVzY3Jvd1VwZGF0ZSAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiBmaW5hbGl6ZSkgPT09IGZpbmFsaXplKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkZpbmFsaXplIiAvKiBGaW5hbGl6ZSAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiB1c2VyX2RlZmluZSkgPT09IHVzZXJfZGVmaW5lKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIlVzZXJEZWZpbmVkRGVmYXVsdCIgLyogVXNlckRlZmluZWREZWZhdWx0ICovKTsKICB9CiAgaWYgKChmbGFncyAmIHRlbXBsYXRlKSA9PT0gdGVtcGxhdGUpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiVGVtcGxhdGVDb2x1bW5FU0U5OCIgLyogVGVtcGxhdGVDb2x1bW5FU0U5OCAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiBkZWxldGVfemVybykgPT09IGRlbGV0ZV96ZXJvKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIkRlbGV0ZU9uWmVybyIgLyogRGVsZXRlT25aZXJvICovKTsKICB9CiAgaWYgKChmbGFncyAmIHByaW1hcnkpID09PSBwcmltYXJ5KSB7CiAgICBmbGFnc19kYXRhLnB1c2goIlByaW1hcnlJbmRleFBsYWNlaG9sZGVyIiAvKiBQcmltYXJ5SW5kZXhQbGFjZWhvbGRlciAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiBjb21wcmVzc2VkKSA9PT0gY29tcHJlc3NlZCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJDb21wcmVzc2VkIiAvKiBDb21wcmVzc2VkICovKTsKICB9CiAgaWYgKChmbGFncyAmIGVuY3J5cHRlZCkgPT09IGVuY3J5cHRlZCkgewogICAgZmxhZ3NfZGF0YS5wdXNoKCJFbmNyeXB0ZWQiIC8qIEVuY3J5cHRlZCAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiB2ZXJzaW9uZWQpID09PSB2ZXJzaW9uZWQpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiVmVyc2lvbmVkIiAvKiBWZXJzaW9uZWQgKi8pOwogIH0KICBpZiAoKGZsYWdzICYgZGVsZXRlZCkgPT09IGRlbGV0ZWQpIHsKICAgIGZsYWdzX2RhdGEucHVzaCgiRGVsZXRlZCIgLyogRGVsZXRlZCAqLyk7CiAgfQogIGlmICgoZmxhZ3MgJiB2ZXJzaW9uX2FkZCkgPT09IHZlcnNpb25fYWRkKSB7CiAgICBmbGFnc19kYXRhLnB1c2goIlZlcnNpb25lZEFkZCIgLyogVmVyc2lvbmVkQWRkICovKTsKICB9CiAgcmV0dXJuIGZsYWdzX2RhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIkM6XFxXaW5kb3dzXFxzZWN1cml0eVxcZGF0YWJhc2VcXHNlY2VkaXQuc2RiIjsKICBjb25zdCB0YWJsZSA9ICJTbVRibFNtcCI7CiAgY29uc3QgcmVzdWx0cyA9IGNhdGFsb2dJbmZvKHBhdGgpOwogIGlmIChyZXN1bHRzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGNvbnN0IGluZm8gPSB0YWJsZUluZm8ocmVzdWx0cywgdGFibGUpOwogIGlmIChpbmZvIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGNvbnN0IHBhZ2VzID0gZ2V0UGFnZXMocGF0aCwgaW5mby50YWJsZV9wYWdlKTsKICBpZiAocGFnZXMgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgY29uc3QgZmFrZSA9IHsgInRlc3QiOiB0cnVlIH07CiAgY29uc3QgX2RhdGEgPSBsb29rdXBNZXRhZGF0YShwYXRoLCBpbmZvLCBwYWdlcywgZmFrZSk7CiAgY29uc3QgX2ZpbHRlciA9IGdldFNvbWVDb2x1bW5zKHBhdGgsIGluZm8pOwp9CmZ1bmN0aW9uIGxvb2t1cE1ldGFkYXRhKHBhdGgsIG1ldGFfdGFibGUsIG1ldGFfcGFnZXMsIGZpbHRlcl92YWx1ZXMpIHsKICBjb25zdCBwYWdlX2xpbWl0ID0gMTsKICBsZXQgcGFnZV9jaHVuayA9IFtdOwogIGNvbnN0IHRvdGFsX3Jlc3VsdHMgPSB7ICJTbVRibFNtcCI6IFtdIH07CiAgZm9yIChjb25zdCBwYWdlIG9mIG1ldGFfcGFnZXMpIHsKICAgIGlmIChwYWdlID09PSAwKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgcGFnZV9jaHVuay5wdXNoKHBhZ2UpOwogICAgaWYgKHBhZ2VfY2h1bmsubGVuZ3RoICE9IHBhZ2VfbGltaXQpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCByZXN1bHRzID0gZ2V0RmlsdGVyZWRSb3dzKAogICAgICBwYXRoLAogICAgICBwYWdlX2NodW5rLAogICAgICBtZXRhX3RhYmxlLAogICAgICAiU21UYmxTbXAiLAogICAgICAiV29ya0lEIiwKICAgICAgZmlsdGVyX3ZhbHVlcwogICAgKTsKICAgIGlmIChyZXN1bHRzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgdG90YWxfcmVzdWx0c1siU21UYmxTbXAiXSA9IHRvdGFsX3Jlc3VsdHNbIlNtVGJsU21wIl0uY29uY2F0KHJlc3VsdHNbIlNtVGJsU21wIl0pOwogICAgcGFnZV9jaHVuayA9IFtdOwogICAgYnJlYWs7CiAgfQogIHJldHVybiB0b3RhbF9yZXN1bHRzOwp9CmZ1bmN0aW9uIGdldFNvbWVDb2x1bW5zKHBhdGgsIGluZm8pIHsKICBjb25zdCBwYWdlcyA9IGdldFBhZ2VzKHBhdGgsIGluZm8udGFibGVfcGFnZSk7CiAgaWYgKHBhZ2VzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGNvbnN0IGNvbHVtbnMgPSBbIkRvY3VtZW50SUQiLCAiTGFzdE1vZGlmaWVkIiwgIkZpbGVOYW1lIl07CiAgY29uc3QgX2RhdGEgPSBkdW1wVGFibGVDb2x1bW5zKHBhdGgsIHBhZ2VzLCBpbmZvLCAiU21UYmxTbXAiLCBjb2x1bW5zKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("ese_column_filter"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}

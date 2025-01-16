use serde_json::Value;

pub(crate) fn check_meta(data: &mut Value, entries: &mut Vec<Value>) -> Option<()> {
    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("collection_metadata") {
            has_meta = value.clone();
        }
    }
    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["collection_metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(entries);

    Some(())
}

use crate::artifacts::os::macos::plist::property_list::get_dictionary;
use log::warn;
use plist::{Dictionary, Value};

// Get the Vec of dictionaries value from the dictionary
pub(crate) fn get_dictionary_values(dict_data: Value) -> Vec<Dictionary> {
    let mut dictionary_vec = Vec::new();
    if let Some(data) = dict_data.into_array() {
        for value in data {
            let dictionary_value = get_dictionary(&value).unwrap_or_default();
            dictionary_vec.push(dictionary_value);
        }
        dictionary_vec
    } else {
        warn!("No dictionary array in PLIST file");
        dictionary_vec
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::macos::emond::util::get_dictionary_values;
    use plist::{Dictionary, Value};

    #[test]
    fn test_get_dictionary_values() {
        let test: Value = Value::Dictionary(Dictionary::new());
        let test_array: Value = Value::Array(vec![test]);
        let results = get_dictionary_values(test_array);

        assert_eq!(results.len(), 1);
    }
}

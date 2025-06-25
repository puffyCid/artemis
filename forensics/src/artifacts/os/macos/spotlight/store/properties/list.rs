use log::warn;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Extract list data associated with Spotlight property
pub(crate) fn extract_list(
    categories: &HashMap<usize, String>,
    indexes1: &HashMap<usize, Vec<u32>>,
    indexes2: &HashMap<usize, Vec<u32>>,
    list_value: usize,
    prop_type: u8,
) -> Value {
    let item_kind = 3;
    let tree_kind = 2;

    let mut value = Value::Null;

    if (prop_type & item_kind) == item_kind {
        value = extract_categories(categories, indexes2, list_value);
    } else if (prop_type & tree_kind) == tree_kind {
        value = extract_categories(categories, indexes1, list_value);
    } else {
        let cat_option = categories.get(&list_value);
        if cat_option.is_some() {
            let cat = cat_option.unwrap_or(&String::new()).clone();
            value = json!(cat);
            return value;
        }
        warn!("[spotlight] Cannot determine category for attribute list.");
    }

    value
}

/// Get categories associated with list
fn extract_categories(
    categories: &HashMap<usize, String>,
    indexes: &HashMap<usize, Vec<u32>>,
    list_value: usize,
) -> Value {
    let value = indexes.get(&list_value);
    if value.is_none() {
        warn!(
            "[spotlight] No value found in indexes data. Cannot determine category for Attribute list"
        );
        return Value::Null;
    }

    let index_value = value.unwrap_or(&Vec::new()).clone();
    let mut category_vec = Vec::new();
    for index in index_value {
        let category = categories.get(&(index as usize));
        if category.is_none() {
            continue;
        }

        let category_value = category.unwrap_or(&String::new()).clone();
        category_vec.push(category_value);
    }
    json!(category_vec)
}

#[cfg(test)]
mod tests {
    use super::{extract_categories, extract_list};
    use std::collections::HashMap;

    #[test]
    fn test_extract_list() {
        let mut indexes1 = HashMap::new();
        indexes1.insert(1, vec![1]);

        let indexes2 = indexes1.clone();
        let mut categories = HashMap::new();
        categories.insert(1, String::from("fakeme"));
        let list_value = 1;
        let prop_type = 2;

        let result = extract_list(&categories, &indexes1, &indexes2, list_value, prop_type);
        assert_eq!(result.as_array().unwrap()[0].as_str().unwrap(), "fakeme");
    }

    #[test]
    fn test_extract_categories() {
        let mut indexes1 = HashMap::new();
        indexes1.insert(1, vec![1]);

        let mut categories = HashMap::new();
        categories.insert(1, String::from("fakeme"));
        let list_value = 1;

        let result = extract_categories(&categories, &indexes1, list_value);
        assert_eq!(result.as_array().unwrap()[0].as_str().unwrap(), "fakeme");
    }

    #[test]
    fn test_extract_list_null() {
        let mut indexes1 = HashMap::new();
        indexes1.insert(1, vec![1]);

        let indexes2 = indexes1.clone();
        let mut categories = HashMap::new();
        categories.insert(11, String::from("fakeme"));
        let list_value = 1;
        let prop_type = 77;

        let result = extract_list(&categories, &indexes1, &indexes2, list_value, prop_type);
        assert_eq!(result.as_null().unwrap(), ());
    }
}

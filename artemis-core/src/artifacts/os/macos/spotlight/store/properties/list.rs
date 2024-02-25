use crate::{
    artifacts::os::macos::spotlight::store::property::parse_variable_size,
    utils::time::cocoatime_to_unixepoch,
};
use nom::{bytes::complete::take, number::complete::le_f64};
use serde_json::{json, Value};
use std::{collections::HashMap, mem::size_of};

pub(crate) fn extract_list(
    categories: &HashMap<usize, String>,
    indexes1: &HashMap<usize, Vec<u32>>,
    indexes2: &HashMap<usize, Vec<u32>>,
    list_value: &usize,
    prop_type: &u8,
) -> Value {
    let item_kind = 3;
    let tree_kind = 2;

    let mut value = Value::Null;

    if (prop_type & &item_kind) == item_kind {
        value = extract_categories(categories, indexes2, list_value);
    } else if (prop_type & &tree_kind) == tree_kind {
        value = extract_categories(categories, indexes1, list_value);
    } else {
        let cat_option = categories.get(list_value);
        if cat_option.is_some() {
            let cat = cat_option.unwrap_or(&String::new()).clone();
            value = json!(cat);
            return value;
        }
        panic!("[spotlight] Cannot determine category for attribute list.");
    }

    value
}

fn extract_categories(
    categories: &HashMap<usize, String>,
    indexes: &HashMap<usize, Vec<u32>>,
    list_value: &usize,
) -> Value {
    let value = indexes.get(list_value);
    if value.is_none() {
        panic!("[spotlight] No value found in indexes data. Cannot determine category for Attribute list");
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

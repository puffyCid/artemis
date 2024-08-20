use super::{
    context::{get_property_type, PropertyType},
    header::HeapPageMap,
    properties::{property_id_to_name, PropertyName},
};
use crate::{
    artifacts::os::windows::outlook::pages::btree::NodeLevel,
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use nom::bytes::complete::take;
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct PropertyContext {
    name: Vec<PropertyName>,
    property_type: PropertyType,
    property_number: u16,
    reference: u32,
    value: Value,
}

pub(crate) fn parse_property_context<'a>(
    data: &'a [u8],
    map: &HeapPageMap,
    level: &NodeLevel,
) -> nom::IResult<&'a [u8], Vec<PropertyContext>> {
    if level == &NodeLevel::BranchNode {
        panic!("branch property context!");
    }

    let prop_offset = 20;

    let mut prop_data_size = 0;
    for (key, value) in map.allocation_table.iter().enumerate() {
        // Only loop until we reach the allocation acount
        if key == map.allocation_count as usize {
            break;
        }
        // Should always be the 2nd value
        if value != &prop_offset {
            continue;
        }

        if let Some(next_value) = map.allocation_table.get(key + 1) {
            prop_data_size = next_value - prop_offset;
        }
    }

    let (input, mut props) = take(prop_data_size)(data)?;
    let prop_entry_size = 8;
    if props.len() % prop_entry_size != 0 {
        panic!("props definitions should always be a multiple of 8 bytes?! {prop_data_size}");
    }

    let prop_count = props.len() / prop_entry_size;
    let mut count = 0;

    let mut props_vec = Vec::new();

    let prop_embedded = vec![
        PropertyType::Int16,
        PropertyType::Int32,
        PropertyType::Float32,
        PropertyType::ErrorCode,
        PropertyType::Bool,
    ];
    while count < prop_count {
        let (remaining, prop_id) = nom_unsigned_two_bytes(props, Endian::Le)?;
        let (remaining, prop_type_num) = nom_unsigned_two_bytes(remaining, Endian::Le)?;
        let (remaining, value_reference) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let name = property_id_to_name(&format!("0x{:04x?}_0x{:04x?}", prop_id, prop_type_num));

        props = remaining;
        count += 1;

        let mut prop = PropertyContext {
            name,
            property_type: get_property_type(&prop_type_num),
            property_number: prop_type_num,
            reference: value_reference,
            value: Value::Null,
        };

        // If the property value is less than 4 bytes then the value is stored with the defition
        if prop_embedded.contains(&prop.property_type) && prop.reference != 0 {
            prop.value = serde_json::to_value(value_reference).unwrap_or(Value::Null)
        }

        props_vec.push(prop);
    }
    println!("{props_vec:?}");

    let node_offset = 12;
    let prop_data = 2;

    // Now go through allocation table again and get the sizes for all properties that have data larger than 4 bytes
    for (key, value) in map.allocation_table.iter().enumerate() {
        // Only loop until we reach the allocation acount
        if key == map.allocation_count as usize {
            break;
        } else if value == &prop_offset || value == &node_offset {
            continue;
        }

        if let Some(next_value) = map.allocation_table.get(key + 1) {
            let data_size = next_value - value;

            // Binary, string, multi will use data_size
            // Everything we can get the data based on type (ex: 8 byte value is nom_unsigned_eight_bytes)
            // Is this right????
            for prop in props_vec.iter_mut() {
                if prop.reference != 0 && prop.value == Value::Null {
                    println!("size: {data_size}");
                    println!("offset: {value}");
                    let (_, prop_value) = get_property_data(
                        data,
                        data_size,
                        &prop.property_type,
                        *value - prop_offset,
                    )?;
                    prop.value = prop_value;
                    break;
                }
            }
        }
    }

    Ok((data, props_vec))
}

pub(crate) fn get_property_data<'a>(
    data: &'a [u8],
    size: u16,
    prop_type: &PropertyType,
    offset: u16,
) -> nom::IResult<&'a [u8], Value> {
    let (input, _) = take(offset)(data)?;
    let (input, prop_data) = take(size)(input)?;
    let mut value = Value::Null;

    match prop_type {
        PropertyType::Int16 => {
            let (_, prop_value) = nom_unsigned_two_bytes(prop_data, Endian::Le)?;
            value = serde_json::to_value(&prop_value).unwrap_or_default();
        }
        PropertyType::Int32 => {
            let (_, prop_value) = nom_unsigned_four_bytes(prop_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Float32 => todo!(),
        PropertyType::Float64 => todo!(),
        PropertyType::Currency => todo!(),
        PropertyType::FloatTime => todo!(),
        PropertyType::ErrorCode => todo!(),
        PropertyType::Bool => {
            let (_, prop_value) = nom_unsigned_one_byte(prop_data, Endian::Le)?;
            let prop_bool = if prop_value != 0 { true } else { false };
            value = serde_json::to_value(prop_bool).unwrap_or_default();
        }
        PropertyType::Int64 => {
            let (_, prop_value) = nom_unsigned_eight_bytes(prop_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::String => todo!(),
        PropertyType::String8 => todo!(),
        PropertyType::Time => {
            let (_, prop_value) = nom_unsigned_eight_bytes(prop_data, Endian::Le)?;
            let timestamp = filetime_to_unixepoch(&prop_value);
            value = serde_json::to_value(unixepoch_to_iso(&timestamp)).unwrap_or_default();
        }
        PropertyType::Guid => todo!(),
        PropertyType::ServerId => todo!(),
        PropertyType::Restriction => todo!(),
        PropertyType::Binary | PropertyType::Unknown => {
            value = serde_json::to_value(base64_encode_standard(prop_data)).unwrap_or_default();
        }
        PropertyType::MultiIn16 => todo!(),
        PropertyType::MultiInt32 => todo!(),
        PropertyType::MultiFloat32 => todo!(),
        PropertyType::MultiFloat64 => todo!(),
        PropertyType::MultiCurrency => todo!(),
        PropertyType::MultiFloatTime => todo!(),
        PropertyType::MultiInt64 => todo!(),
        PropertyType::MultiString => todo!(),
        PropertyType::MultiString8 => todo!(),
        PropertyType::MultiTime => todo!(),
        PropertyType::MultiGuid => todo!(),
        PropertyType::MultiBinary => todo!(),
        PropertyType::Unspecified => todo!(),
        PropertyType::Null => todo!(),
        PropertyType::Object => todo!(),
        PropertyType::RuleAction => todo!(),
    };

    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use super::parse_property_context;
    use crate::artifacts::os::windows::outlook::{
        pages::btree::NodeLevel,
        tables::{context::PropertyType, header::HeapPageMap, properties::PropertyName},
    };

    #[test]
    fn test_parse_property_context_root_folder() {
        let test = [
            1, 48, 31, 0, 0, 0, 0, 0, 4, 48, 31, 0, 0, 0, 0, 0, 7, 48, 64, 0, 128, 0, 0, 0, 8, 48,
            64, 0, 96, 0, 0, 0, 2, 54, 3, 0, 0, 0, 0, 0, 3, 54, 3, 0, 0, 0, 0, 0, 10, 54, 11, 0, 1,
            0, 0, 0, 228, 63, 11, 0, 0, 0, 0, 0, 229, 63, 11, 0, 0, 0, 0, 0, 20, 102, 2, 1, 160, 0,
            0, 0, 56, 102, 3, 0, 2, 0, 0, 0, 57, 102, 3, 0, 251, 5, 0, 0, 112, 189, 150, 244, 111,
            225, 218, 1, 112, 189, 150, 244, 111, 225, 218, 1, 70, 53, 70, 86, 3, 0, 0, 0, 177, 0,
            0, 0, 106, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 142, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 27, 1, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 104, 0, 0, 0, 8, 0, 0, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134,
            11, 232, 66, 98, 69, 158, 194, 6, 0, 0, 0, 0, 1, 12, 0, 3, 0, 0, 0, 0, 0, 0, 0, 94,
            178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 82, 0, 0, 0, 0, 0,
            1, 0, 3, 34, 183, 166, 197, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66,
            98, 69, 158, 194, 82, 0, 0, 0, 0, 0, 1, 0, 3, 34, 183, 166, 197, 0, 91, 220, 80, 80, 0,
            47, 111, 61, 70, 105, 114, 115, 116, 32, 79, 114, 103, 97, 110, 105, 122, 97, 116, 105,
            111, 110, 47, 111, 117, 61, 69, 120, 99, 104, 97, 110, 103, 101, 32, 65, 100, 109, 105,
            110, 105, 115, 116, 114, 97, 116, 105, 118, 101, 32, 71, 114, 111, 117, 112, 40, 70,
            89, 68, 73, 66, 79, 72, 70, 50, 51, 83, 80, 68, 76, 84, 41, 47, 99, 110, 61, 82, 101,
            99, 105, 112, 105, 101, 110, 116, 115, 47, 99, 110, 61, 48, 48, 48, 51, 66, 70, 70, 68,
            51, 57, 56, 69, 69, 66, 48, 49, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232,
            66, 98, 69, 158, 194, 1, 0, 1, 0, 3, 0, 0, 1, 82, 9, 18, 66, 27, 4, 66, 39, 253, 66,
            77, 193, 66, 92, 23, 80, 3, 133, 158, 143, 82, 134, 135, 80, 80, 3, 3, 20, 32, 1, 30,
            82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80, 80, 0, 23, 80, 3, 133, 158, 143, 82,
            134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80,
            80, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 1, 0, 1,
            0, 3, 0, 0, 1, 82, 9, 18, 66, 27, 4, 66, 39, 253, 66, 77, 193, 66, 92, 23, 80, 3, 133,
            158, 143, 82, 134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219,
            220, 80, 80, 80, 0, 0, 5, 0, 0, 0, 12, 0, 20, 0, 116, 0, 124, 0, 132, 0, 69, 2,
        ];

        let map = HeapPageMap {
            allocation_count: 5,
            free: 0,
            allocation_table: vec![12, 20, 116, 124, 132, 561],
        };

        let (_, result) = parse_property_context(&test, &map, &NodeLevel::LeafNode).unwrap();
        println!("{result:?}");
        assert_eq!(result[2].property_type, PropertyType::Time);
        assert_eq!(result[2].name, vec![PropertyName::PidTagCreationTime]);
        assert_eq!(
            result[2].value.as_str().unwrap(),
            "2024-07-29T04:29:52.000Z"
        );

        assert_eq!(result[9].property_type, PropertyType::Binary);
        assert_eq!(result[9].name, vec![PropertyName::Unknown]);
        assert_eq!(result[9].value.as_str().unwrap(), "RjVGVgMAAACxAAAAagAAAAAAAAAAAAAAjgAAAB4AAAAAAAAAAAAAABsBAABEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaAAAAAgAAABespa0g00oQoYL6EJiRZ7CBgAAAAABDAADAAAAAAAAAF6ylrSDTShChgvoQmJFnsJSAAAAAAABAAMit6bFAF6ylrSDTShChgvoQmJFnsJSAAAAAAABAAMit6bFAFvcUFAAL289Rmlyc3QgT3JnYW5pemF0aW9uL291PUV4Y2hhbmdlIEFkbWluaXN0cmF0aXZlIEdyb3VwKEZZRElCT0hGMjNTUERMVCkvY249UmVjaXBpZW50cy9jbj0wMDAzQkZGRDM5OEVFQjAxAF6ylrSDTShChgvoQmJFnsIBAAEAAwAAAVIJEkIbBEIn/UJNwUJcF1ADhZ6PUoaHUFADAxQgAR5SuLtQAVtS29xQUFAAF1ADhZ6PUoaHUFADAxQgAR5SuLtQAVtS29xQUFAAXrKWtINNKEKGC+hCYkWewgEAAQADAAABUgkSQhsEQif9Qk3BQlwXUAOFno9ShodQ");
    }
}

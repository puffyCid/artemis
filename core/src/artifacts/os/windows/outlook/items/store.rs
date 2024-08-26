use crate::artifacts::os::windows::outlook::tables::{
    property::PropertyContext, table::property_context_table,
};

pub(crate) fn parse_message_store(data: &[u8]) -> nom::IResult<&[u8], Vec<PropertyContext>> {
    property_context_table(data)
}

#[cfg(test)]
mod tests {
    use super::parse_message_store;
    use crate::artifacts::os::windows::outlook::tables::{
        context::PropertyType, properties::PropertyName,
    };

    #[test]
    fn test_parse_message_store() {
        let test = [
            174, 1, 236, 188, 32, 0, 0, 0, 0, 0, 0, 0, 181, 2, 6, 0, 64, 0, 0, 0, 92, 14, 11, 0, 1,
            0, 0, 0, 249, 15, 2, 1, 96, 0, 0, 0, 1, 48, 31, 0, 0, 0, 0, 0, 22, 52, 2, 1, 32, 1, 0,
            0, 21, 102, 72, 0, 160, 0, 0, 0, 31, 102, 20, 0, 0, 1, 0, 0, 32, 102, 3, 0, 249, 1, 0,
            0, 51, 102, 11, 0, 1, 0, 0, 0, 109, 102, 3, 0, 0, 140, 0, 0, 250, 102, 3, 0, 17, 0, 14,
            0, 252, 102, 3, 0, 62, 175, 24, 0, 255, 103, 3, 0, 255, 255, 255, 255, 4, 124, 2, 1,
            192, 0, 0, 0, 6, 124, 31, 16, 64, 1, 0, 0, 7, 124, 2, 1, 128, 0, 0, 0, 12, 124, 3, 0,
            0, 0, 0, 0, 13, 124, 20, 0, 224, 0, 0, 0, 17, 124, 11, 0, 1, 0, 0, 0, 19, 124, 3, 0, 4,
            55, 18, 0, 13, 121, 253, 85, 247, 74, 143, 77, 141, 121, 129, 146, 72, 127, 210, 0, 1,
            0, 0, 0, 186, 86, 57, 234, 168, 210, 22, 74, 160, 69, 90, 22, 243, 172, 249, 176, 1, 8,
            0, 0, 0, 252, 0, 0, 0, 0, 0, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66,
            98, 69, 158, 194, 0, 3, 34, 185, 231, 130, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 91, 215, 45, 167, 52, 215, 220, 68, 175, 222, 60, 208,
            93, 32, 138, 165, 70, 100, 212, 225, 117, 185, 224, 64, 185, 193, 109, 232, 93, 23, 22,
            10, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 3, 34, 185, 229, 137, 0, 0, 0, 0, 91,
            215, 45, 167, 52, 215, 220, 68, 175, 222, 60, 208, 93, 32, 138, 165, 1, 0, 94, 178,
            150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 0, 3, 34, 185, 229, 131,
            0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 92, 0, 79, 0, 102, 0, 102, 0, 108, 0, 105, 0, 110, 0,
            101, 0, 32, 0, 71, 0, 108, 0, 111, 0, 98, 0, 97, 0, 108, 0, 32, 0, 65, 0, 100, 0, 100,
            0, 114, 0, 101, 0, 115, 0, 115, 0, 32, 0, 76, 0, 105, 0, 115, 0, 116, 0, 10, 0, 0, 0,
            12, 0, 20, 0, 172, 0, 188, 0, 12, 1, 28, 1, 48, 1, 56, 1, 64, 1, 110, 1, 174, 1,
        ];
        let (_, store) = parse_message_store(&test).unwrap();
        println!("{store:?}");
        assert_eq!(store.len(), 19);
        assert_eq!(store[3].name, vec![PropertyName::Unknown]);
        assert_eq!(store[3].property_type, PropertyType::Binary);
        assert_eq!(
            store[3].value.as_str().unwrap(),
            "AAAAAFvXLac019xEr9480F0giqUBAF6ylrSDTShChgvoQmJFnsIAAyK55YMAAA=="
        );

        assert_eq!(store[13].name, vec![PropertyName::Unknown]);
        assert_eq!(store[13].property_type, PropertyType::MultiString);
        assert_eq!(
            store[13].value.as_array().unwrap(),
            &vec![serde_json::to_value("\\Offline Global Address List").unwrap()]
        );
    }
}

use crate::utils::uuid::format_guid_le_bytes;
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ObjectId {
    droid_file_id: String,
    birth_droid_volume_id: String,
    birth_droid_file_id: String,
    birth_droid_domain_id: String,
}

impl ObjectId {
    /// Extract object IDs from attribute
    pub(crate) fn parse_object_id(data: &[u8]) -> nom::IResult<&[u8], ObjectId> {
        let guid_size: u8 = 16;

        let (input, droid_id) = take(guid_size)(data)?;

        if input.is_empty() {
            let id = ObjectId {
                droid_file_id: format_guid_le_bytes(droid_id),
                birth_droid_file_id: String::new(),
                birth_droid_domain_id: String::new(),
                birth_droid_volume_id: String::new(),
            };

            return Ok((input, id));
        }

        let (input, volume_id) = take(guid_size)(input)?;
        let (input, birth_file_id) = take(guid_size)(input)?;
        let (input, domain_id) = take(guid_size)(input)?;

        let id = ObjectId {
            droid_file_id: format_guid_le_bytes(droid_id),
            birth_droid_volume_id: format_guid_le_bytes(volume_id),
            birth_droid_domain_id: format_guid_le_bytes(domain_id),
            birth_droid_file_id: format_guid_le_bytes(birth_file_id),
        };

        Ok((input, id))
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectId;

    #[test]
    fn test_parse_object_id() {
        let test = [
            224, 40, 178, 130, 84, 215, 183, 74, 184, 24, 222, 128, 94, 186, 203, 148,
        ];
        let (_, result) = ObjectId::parse_object_id(&test).unwrap();

        assert_eq!(result.droid_file_id, "82b228e0-d754-4ab7-b818-de805ebacb94");
    }
}

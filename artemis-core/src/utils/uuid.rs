use log::warn;
use uuid::Uuid;

/// Create a UUID and return as a string
pub(crate) fn generate_uuid() -> String {
    Uuid::new_v4().hyphenated().to_string()
}

#[cfg(target_os = "windows")]
/// Convert little endian bytes to a UUID/GUID string
pub(crate) fn format_guid_le_bytes(data: &[u8]) -> String {
    let guid_size = 16;
    if data.len() != guid_size {
        warn!(
            "[artemis-core] Provided little endian data does not meet GUID size of 16 bytes, got: {}",
            data.len()
        );
        return format!("Not a GUID/UUID: {data:?}");
    }

    let guid_data = data.try_into();
    match guid_data {
        Ok(result) => Uuid::from_bytes_le(result).hyphenated().to_string(),
        Err(_err) => {
            warn!("[artemis-core] Could not convert little endian bytes to a GUID/UUID format: {data:?}");
            format!("Could not convert data: {data:?}")
        }
    }
}

#[cfg(target_os = "macos")]
/// Convert big endian bytes to a UUID/GUID string
pub(crate) fn format_guid_be_bytes(data: &[u8]) -> String {
    let guid_size = 16;
    if data.len() != guid_size {
        warn!(
            "[artemis-core] Provided big endian data does not meet GUID size of 16 bytes, got: {}",
            data.len()
        );
        return format!("Not a GUID/UUID: {data:?}");
    }

    let guid_data = data.try_into();
    match guid_data {
        Ok(result) => Uuid::from_bytes(result).hyphenated().to_string(),
        Err(_err) => {
            warn!(
                "[artemis-core] Could not convert big endian bytes to a GUID/UUID format: {data:?}"
            );
            format!("Could not convert data: {data:?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_uuid;

    #[test]
    fn test_generate_uuid() {
        let result = generate_uuid();
        assert_eq!(result.is_empty(), false);

        let result2 = generate_uuid();
        assert_ne!(result, result2)
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_format_guid_le_bytes() {
        use crate::utils::uuid::format_guid_le_bytes;

        let test_data = [
            17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
        ];
        let guid = format_guid_le_bytes(&test_data);
        assert_eq!(guid, "11111111-1111-1111-1111-111111111111");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_format_bad_guid_le_bytes() {
        use crate::utils::uuid::format_guid_le_bytes;

        let test_data = [17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17];
        let guid = format_guid_le_bytes(&test_data);
        assert_eq!(
            guid,
            "Not a GUID/UUID: [17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17]"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_format_guid_be_bytes() {
        use crate::utils::uuid::format_guid_be_bytes;

        let test_data = [
            118, 176, 112, 103, 44, 205, 62, 212, 191, 187, 89, 4, 99, 208, 235, 224,
        ];
        let guid = format_guid_be_bytes(&test_data);
        assert_eq!(guid, "76b07067-2ccd-3ed4-bfbb-590463d0ebe0")
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_format_bad_guid_be_bytes() {
        use crate::utils::uuid::format_guid_be_bytes;

        let test_data = [
            118, 176, 112, 103, 44, 205, 62, 212, 191, 187, 89, 4, 99, 208, 235, 224, 117,
        ];
        let guid = format_guid_be_bytes(&test_data);
        assert_eq!(guid, "Not a GUID/UUID: [118, 176, 112, 103, 44, 205, 62, 212, 191, 187, 89, 4, 99, 208, 235, 224, 117]")
    }
}

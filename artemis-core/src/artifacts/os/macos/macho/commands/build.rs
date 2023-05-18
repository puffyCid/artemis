use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Build {
    pub(crate) platform: u32,
    pub(crate) minos: String,
    pub(crate) sdk: String,
    pub(crate) ntools: u32,
}

impl Build {
    /// Parse build system data
    pub(crate) fn parse_build_version(data: &[u8]) -> nom::IResult<&[u8], Build> {
        let (build_data, platform) = nom_unsigned_four_bytes(data, Endian::Le)?;

        let (build_data, minos) = Build::get_versions(build_data)?;
        let (build_data, sdk) = Build::get_versions(build_data)?;
        let (build_data, ntools) = nom_unsigned_four_bytes(build_data, Endian::Le)?;

        let build = Build {
            platform,
            minos,
            sdk,
            ntools,
        };

        Ok((build_data, build))
    }

    /// Get common versioning format. MAJOR.MINOR.PATCH
    pub(crate) fn get_versions(data: &[u8]) -> nom::IResult<&[u8], String> {
        let (build_data, patch) = nom_unsigned_one_byte(data, Endian::Be)?;
        let (build_data, minor) = nom_unsigned_one_byte(build_data, Endian::Be)?;
        let (build_data, major) = nom_unsigned_two_bytes(build_data, Endian::Le)?;

        Ok((build_data, format!("{major}.{minor}.{patch}")))
    }
}

#[cfg(test)]
mod tests {
    use super::Build;

    #[test]
    fn test_parse_build_version() {
        let test_data = [1, 0, 0, 0, 0, 14, 10, 0, 0, 3, 12, 0, 1, 0, 0, 0];
        let (_, result) = Build::parse_build_version(&test_data).unwrap();

        assert_eq!(result.platform, 0x1);
        assert_eq!(result.minos, "10.14.0");
        assert_eq!(result.sdk, "12.3.0");
        assert_eq!(result.ntools, 0x1);
    }

    #[test]
    fn test_get_versions() {
        let test_data = [0, 14, 10, 0];
        let (_, result) = Build::get_versions(&test_data).unwrap();
        assert_eq!(result, "10.14.0");
    }
}

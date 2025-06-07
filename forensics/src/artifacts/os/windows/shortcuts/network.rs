use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    strings::{extract_utf8_string, extract_utf16_string},
};
use common::windows::NetworkProviderType;
use nom::{
    Needed,
    bytes::complete::{take, take_while},
};

#[derive(Debug)]
pub(crate) struct LnkNetwork {
    _size: u32,
    pub(crate) _flags: NetworkFlags,
    name_offset: u32,
    device_offset: u32,
    pub(crate) provider_type: NetworkProviderType,
    unicode_share_name_offset: u32,
    unicode_device_name_offset: u32,
    pub(crate) share_name: String,
    pub(crate) device_name: String,
    pub(crate) unicode_share_name: String,
    pub(crate) unicode_device_name: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum NetworkFlags {
    ValidDevice,
    ValidNetType,
    UnknownFlag,
}

impl LnkNetwork {
    /// Parse network device metadata from `shortcut` data
    pub(crate) fn parse_network(data: &[u8]) -> nom::IResult<&[u8], LnkNetwork> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;

        // Size includes the size itself (4 bytes)
        let adjust_size = 4;
        if size < adjust_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (remaining_input, input) = take(size - adjust_size)(input)?;

        let (input, flag) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, name_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, device_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, provider) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let mut network = LnkNetwork {
            _size: size,
            _flags: LnkNetwork::get_flag_type(&flag),
            name_offset,
            device_offset,
            provider_type: LnkNetwork::get_provider_type(&provider),
            unicode_share_name_offset: 0,
            unicode_device_name_offset: 0,
            share_name: String::new(),
            device_name: String::new(),
            unicode_share_name: String::new(),
            unicode_device_name: String::new(),
        };

        let has_unicode = 20;
        if network.name_offset > has_unicode {
            let (input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            network.unicode_share_name_offset = offset;

            let (_, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            network.unicode_device_name_offset = offset;

            let (name_start, _) = take(network.unicode_share_name_offset)(data)?;
            network.unicode_share_name = extract_utf16_string(name_start);

            let (device_start, _) = take(network.unicode_device_name_offset)(data)?;
            network.unicode_device_name = extract_utf16_string(device_start);
        }

        let (share_name_start, _) = take(network.name_offset)(data)?;
        let end_of_string = 0;
        let (_, shame_name_data) = take_while(|b| b != end_of_string)(share_name_start)?;

        network.share_name = extract_utf8_string(shame_name_data);

        let (device_name_start, _) = take(network.device_offset)(data)?;
        let (_, device_data) = take_while(|b| b != end_of_string)(device_name_start)?;

        network.device_name = extract_utf8_string(device_data);

        Ok((remaining_input, network))
    }

    /// Get flag type for network device
    fn get_flag_type(flag: &u32) -> NetworkFlags {
        match flag {
            1 => NetworkFlags::ValidDevice,
            2 => NetworkFlags::ValidNetType,
            _ => NetworkFlags::UnknownFlag,
        }
    }

    /// Get provider type for network device
    fn get_provider_type(provider: &u32) -> NetworkProviderType {
        match provider {
            0x1a0000 => NetworkProviderType::WnncNetAvid,
            0x1b0000 => NetworkProviderType::WnncNetDocuspace,
            0x1c0000 => NetworkProviderType::WnncNetMangsoft,
            0x1d0000 => NetworkProviderType::WnncNetSernet,
            0x1e0000 => NetworkProviderType::WnncNetRiverFront1,
            0x1f0000 => NetworkProviderType::WnncNetRiverFront2,
            0x200000 => NetworkProviderType::WnncNetDecorb,
            0x210000 => NetworkProviderType::WnncNetProtstor,
            0x220000 => NetworkProviderType::WnncNetFjRedir,
            0x230000 => NetworkProviderType::WnncNetDistinct,
            0x240000 => NetworkProviderType::WnncNetTwins,
            0x250000 => NetworkProviderType::WnncNetRdr2Sample,
            0x260000 => NetworkProviderType::WnncNetCsc,
            0x270000 => NetworkProviderType::WnncNet3In1,
            0x290000 => NetworkProviderType::WnncNetExtendNet,
            0x2a0000 => NetworkProviderType::WnncNetStac,
            0x2b0000 => NetworkProviderType::WnncNetFoxbat,
            0x2c0000 => NetworkProviderType::WnncNetYahoo,
            0x2d0000 => NetworkProviderType::WnncNetExifs,
            0x2e0000 => NetworkProviderType::WnncNetDav,
            0x2f0000 => NetworkProviderType::WnncNetKnoware,
            0x300000 => NetworkProviderType::WnncNetObjectDire,
            0x310000 => NetworkProviderType::WnncNetMasfax,
            0x320000 => NetworkProviderType::WnncNetHobNfs,
            0x330000 => NetworkProviderType::WnncNetShiva,
            0x340000 => NetworkProviderType::WnncNetIbmal,
            0x350000 => NetworkProviderType::WnncNetLock,
            0x360000 => NetworkProviderType::WnncNetTermsrv,
            0x370000 => NetworkProviderType::WnncNetSrt,
            0x380000 => NetworkProviderType::WnncNetQuincy,
            0x390000 => NetworkProviderType::WnncNetOpenafs,
            0x3a0000 => NetworkProviderType::WnncNetAvid1,
            0x3b0000 => NetworkProviderType::WnncNetDfs,
            0x3c0000 => NetworkProviderType::WnncNetKwnp,
            0x3d0000 => NetworkProviderType::WnncNetZenworks,
            0x3e0000 => NetworkProviderType::WnncNetDriveOnWeb,
            0x3f0000 => NetworkProviderType::WnncNetVmware,
            0x400000 => NetworkProviderType::WnncNetRsfx,
            0x410000 => NetworkProviderType::WnncNetMfiles,
            0x420000 => NetworkProviderType::WnncNetMsNfs,
            0x430000 => NetworkProviderType::WnncNetGoogle,
            _ => NetworkProviderType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LnkNetwork;
    use crate::artifacts::os::windows::shortcuts::network::{NetworkFlags, NetworkProviderType};

    #[test]
    fn test_parse_network() {
        let test = [
            43, 0, 0, 0, 3, 0, 0, 0, 20, 0, 0, 0, 40, 0, 0, 0, 0, 0, 37, 0, 92, 92, 86, 66, 111,
            120, 83, 118, 114, 92, 68, 111, 119, 110, 108, 111, 97, 100, 115, 0, 90, 58, 0,
        ];
        let (_, results) = LnkNetwork::parse_network(&test).unwrap();
        assert_eq!(results._size, 43);
        assert_eq!(results._flags, NetworkFlags::UnknownFlag);
        assert_eq!(results.name_offset, 20);
        assert_eq!(results.device_offset, 40);
        assert_eq!(
            results.provider_type,
            NetworkProviderType::WnncNetRdr2Sample
        );
        assert_eq!(results.unicode_share_name_offset, 0);
        assert_eq!(results.unicode_device_name_offset, 0);
        assert_eq!(results.share_name, "\\\\VBoxSvr\\Downloads");
        assert_eq!(results.device_name, "Z:");
        assert_eq!(results.unicode_share_name, "");
        assert_eq!(results.unicode_device_name, "");
    }

    #[test]
    fn test_get_provider_type() {
        let test = 1;
        let result = LnkNetwork::get_flag_type(&test);
        assert_eq!(result, NetworkFlags::ValidDevice);
    }

    #[test]
    fn test_get_flag_type() {
        let test = 0x3f0000;
        let result = LnkNetwork::get_provider_type(&test);
        assert_eq!(result, NetworkProviderType::WnncNetVmware);
    }
}

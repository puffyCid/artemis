use super::{
    crimson::parse_crimson,
    data::{ManifestData, parse_manifest_data},
    defintion::{Definition, parse_definition},
    maps::{MapInfo, parse_map},
    provider::parse_provider,
    task::{Task, parse_task},
};
use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};
use log::warn;
use nom::bytes::complete::take;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ManifestTemplate {
    /**Offset to start of Provider */
    pub(crate) offset: u32,
    pub(crate) element_offsets: Vec<u32>,
    pub(crate) channels: Vec<ManifestData>,
    pub(crate) keywords: Vec<ManifestData>,
    pub(crate) opcodes: Vec<ManifestData>,
    pub(crate) levels: Vec<ManifestData>,
    pub(crate) maps: Vec<MapInfo>,
    pub(crate) tasks: Vec<Task>,
    pub(crate) definitions: HashMap<String, Definition>,
}

/// Parse `WEVT_TEMPLATE` resource
pub(crate) fn parse_manifest(
    data: &[u8],
) -> nom::IResult<&[u8], HashMap<String, ManifestTemplate>> {
    let (_, mut manifests) = parse_crimson(data)?;

    for value in manifests.values_mut() {
        let (provider_start, _) = take(value.offset)(data)?;
        let (_, offsets) = parse_provider(provider_start)?;
        value.element_offsets = offsets;
        for offset in &value.element_offsets {
            let (element_start, _) = take(*offset)(data)?;
            let (_, sig) = nom_unsigned_four_bytes(element_start, Endian::Le)?;

            let sig_type = get_sig_type(&sig);

            match sig_type {
                SigType::Chan => {
                    let (_, channels) = parse_manifest_data(data, element_start, &sig_type)?;
                    value.channels = channels;
                }
                SigType::Ttbl | SigType::Evta | SigType::Priva => continue,
                SigType::Opco => {
                    let (_, opcodes) = parse_manifest_data(data, element_start, &sig_type)?;
                    value.opcodes = opcodes;
                }
                SigType::Levl => {
                    let (_, levels) = parse_manifest_data(data, element_start, &sig_type)?;
                    value.levels = levels;
                }
                SigType::Task => {
                    let (_, tasks) = parse_task(data, element_start)?;
                    value.tasks = tasks;
                }
                SigType::Keyw => {
                    let (_, keywords) = parse_manifest_data(data, element_start, &sig_type)?;
                    value.keywords = keywords;
                }
                SigType::Evnt => {
                    let (_, definitions) = parse_definition(data, element_start)?;
                    value.definitions = definitions;
                }
                SigType::Maps => {
                    let (_, maps) = parse_map(data, element_start)?;
                    value.maps = maps;
                }
                SigType::Unknown => warn!("[eventlogs] Unknown manifest sig: {sig}"),
            }
        }
    }

    Ok((&[], manifests))
}

#[derive(Debug, PartialEq)]
pub(crate) enum SigType {
    Chan,
    Evnt,
    Keyw,
    Levl,
    Maps,
    Opco,
    Task,
    Ttbl,
    Priva,
    Evta,
    Unknown,
}

/// Get data signature
fn get_sig_type(sig: &u32) -> SigType {
    match sig {
        0x4e414843 => SigType::Chan,
        0x4c425454 => SigType::Ttbl,
        1096176208 => SigType::Priva,
        0x4f43504f => SigType::Opco,
        0x4c56454c => SigType::Levl,
        0x4b534154 => SigType::Task,
        0x5759454b => SigType::Keyw,
        0x544e5645 => SigType::Evnt,
        0x5350414d => SigType::Maps,
        0x41545645 => SigType::Evta,
        _ => SigType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::wevt::{SigType, get_sig_type},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_manifest() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, manifest) = parse_manifest(&data).unwrap();

        let value = manifest
            .get("9799276c-fb04-47e8-845e-36946045c218")
            .unwrap();
        assert_eq!(value.offset, 36);
        assert_eq!(value.definitions.len(), 16);
        assert_eq!(value.keywords.len(), 22);
        assert_eq!(value.opcodes.len(), 8);
    }

    #[test]
    fn test_get_sig_type() {
        assert_eq!(get_sig_type(&0), SigType::Unknown);
    }

    #[test]
    fn test_parse_manifest_userdata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/cbsmsg_wevt.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, manifest) = parse_manifest(&data).unwrap();

        let value = manifest
            .get("bd12f3b8-fc40-4a61-a307-b7a013a069c1")
            .unwrap();

        assert_eq!(value.offset, 36);
        assert_eq!(value.maps.len(), 1);
        assert_eq!(
            value.maps[0].data.get(&5112).unwrap().message_id,
            -805306358
        );
        assert_eq!(value.definitions.len(), 132);
        assert_eq!(value.keywords.len(), 1);
        assert_eq!(value.opcodes.len(), 3);
    }
}

use super::{
    crimson::parse_crimson,
    data::{parse_manifest_data, ManifestData},
    defintion::{parse_definition, Definition},
    provider::parse_provider,
    table::parse_table,
    task::{parse_task, Task},
    xml::TemplateElement,
};
use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use log::error;
use nom::bytes::complete::take;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct ManifestTemplate {
    /**Offset to start of Provider */
    pub(crate) offset: u32,
    pub(crate) element_offsets: Vec<u32>,
    pub(crate) channels: Vec<ManifestData>,
    pub(crate) keywords: Vec<ManifestData>,
    pub(crate) opcodes: Vec<ManifestData>,
    pub(crate) levels: Vec<ManifestData>,
    pub(crate) templates: Vec<TemplateElement>,
    pub(crate) tasks: Vec<Task>,
    pub(crate) definitions: Vec<Definition>,
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
                SigType::Ttbl => {
                    let (_, templates) = parse_table(element_start)?;
                    value.templates = templates;
                }
                SigType::Opco => {
                    let (_, opcodes) = parse_manifest_data(data, element_start, &sig_type)?;
                    value.opcodes = opcodes;
                }
                SigType::Priva => continue,
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
                    let (_, definitions) = parse_definition(element_start)?;
                    value.definitions = definitions;
                }
                _ => error!("[eventlogs] Unknown manifest sig: {sig}"),
            }
        }
    }

    Ok((&[], manifests))
}

#[derive(Debug)]
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
    Unknown,
}

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
        _ => SigType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;
    use crate::filesystem::files::read_file;
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
        assert_eq!(value.templates.len(), 9);
        assert_eq!(value.opcodes.len(), 8);
    }
}

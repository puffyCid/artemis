use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::{
        nom_helper::{nom_guid, nom_take, nom_u32, nom_u64},
        strings::extract_utf16_string,
    },
};
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

/// Parsed GPT header
///
/// Describes where the partition entry array is located
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GptHeader {
    /// GPT revision
    revision: u32,
    /// GPT header size
    header_size: u32,
    /// GPT header checksum
    header_crc32: u32,
    /// LBA associated with this header
    current_lba: u64,
    /// LBA associated with the backup GPT header
    backup_lba: u64,
    /// First LBA available for partitions
    first_usable_lba: u64,
    /// Last LBA available for partitions
    last_usable_lba: u64,
    /// Unique disk GUID
    disk_guid: Uuid,
    /// Starting LBA of the partition entry array
    partition_entry_lba: u64,
    /// Number of slots in the partition entry array
    partition_entry_count: u32,
    /// Size of one partition entry
    partition_entry_size: u32,
    /// Checksum of the partition entry array
    partition_array_crc32: u32,
}

/// Single GPT partition entry
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GptEntry {
    /// Slot number in the GPT partition entry array
    slot: u32,
    /// GUID associated with the partition type
    partition_type_guid: Uuid,
    /// Unique GUID for this partition
    partition_guid: Uuid,
    /// First LBA occupied by this partition.
    /// Value is inclusive
    start_lba: u64,
    /// Last LBA occupied by this partition.
    /// Value is inclusive
    end_lba: u64,
    /// GPT partition attributes
    attributes: u64,
    /// Partition name (UTF16)
    partition_name: String,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
enum GuidNames {
    Linux,
    Windows,
    Apple,
    Freebsd,
    Netbsd,
    Minix,
    Bios,
    Mbr,
    Efi,
    Unused,
    Illumos,
    Vmware,
    OpenBsd,
    Swap,
    #[default]
    Unknown,
}

impl GptHeader {
    /// Calculate the size of the complete partition entry array
    pub(super) fn partition_array_size(&self) -> AccessorResult<u64> {
        u64::from(self.partition_entry_count)
            .checked_mul(self.partition_entry_size as u64)
            .ok_or_else(|| AccessorError::volume("GPT partition entry array size overflow"))
    }

    /// Calculate the byte offset of the partition entry array
    pub(super) fn partition_array_offset(&self, sector_size: u64) -> AccessorResult<u64> {
        self.partition_entry_lba
            .checked_mul(sector_size)
            .ok_or_else(|| AccessorError::volume("GPT partition entry array offset overflow"))
    }
}

impl GptEntry {
    /// Calculate the partition length
    pub(super) fn byte_length(&self, sector_size: u64) -> AccessorResult<u64> {
        let sector_count = self
            .end_lba
            .checked_sub(self.start_lba)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                AccessorError::volume(format!(
                    "GPT partition {} has an invalid LBA range: {}-{}",
                    self.slot, self.start_lba, self.end_lba
                ))
            })?;

        sector_count
            .checked_mul(sector_size)
            .ok_or_else(|| AccessorError::volume("GPT partition byte length overflow"))
    }
}

pub(super) fn parse_gpt_header(sector: &[u8]) -> AccessorResult<GptHeader> {
    let (input, sig) = nom_u64(sector, "GPT sig is truncated")?;
    if sig != 0x5452415020494645 {
        return Err(AccessorError::volume(format!(
            "Invalid GPT sig {sig:#018x}"
        )));
    }

    let (input, revision) = nom_u32(input, "GPT revision is truncated")?;
    let (input, header_size) = nom_u32(input, "GPT header is truncated")?;
    let (input, header_crc32) = nom_u32(input, "GPT header CRC32 is truncated")?;
    let (input, reserved) = nom_u32(input, "GPT reserved field is truncated")?;
    let (input, current_lba) = nom_u64(input, "GPT current LBA is truncated")?;
    let (input, backup_lba) = nom_u64(input, "GPT backup LBA is truncated")?;

    let (input, first_usable_lba) = nom_u64(input, "GPT first usable LBA is truncated")?;
    let (input, last_usable_lba) = nom_u64(input, "GPT last usable LBA is truncated")?;
    let (input, disk_guid) = nom_guid(input, "GPT disk GUID is truncated")?;
    let (input, partition_entry_lba) = nom_u64(input, "GPT partition entry LBA is truncated")?;
    let (input, partition_entry_count) = nom_u32(input, "GPT partition entry count is truncated")?;

    let (input, partition_entry_size) = nom_u32(input, "GPT partition entry size is truncated")?;
    let (input, partition_array_crc32) = nom_u32(input, "GPT partition array CRC32 is truncated")?;

    if header_size < 92 {
        return Err(AccessorError::volume(format!(
            "Invalid GPT header size {header_size}, minimum size is 92"
        )));
    }

    let available = u32::try_from(sector.len())
        .map_err(|_| AccessorError::volume("GPT header sector length larger than u32::MAX"))?;

    if header_size > available {
        return Err(AccessorError::volume(format!(
            "GPT header size {header_size} larger than available data {available}"
        )));
    }

    if reserved != 0 {
        warn!("GPT reserved field is not zero: {reserved}");
    }

    if partition_entry_size < 128 || !partition_entry_size.is_power_of_two() {
        return Err(AccessorError::volume(format!(
            "Invalid GPT partition entry size {partition_entry_size}"
        )));
    }

    if first_usable_lba > last_usable_lba {
        return Err(AccessorError::volume(format!(
            "Invalid GPT LBA range: {first_usable_lba}-{last_usable_lba}"
        )));
    }

    Ok(GptHeader {
        revision,
        header_size,
        header_crc32,
        current_lba,
        backup_lba,
        first_usable_lba,
        last_usable_lba,
        disk_guid,
        partition_entry_lba,
        partition_entry_count,
        partition_entry_size,
        partition_array_crc32,
    })
}

pub(super) fn parse_gpt_entries(data: &[u8], header: &GptHeader) -> AccessorResult<Vec<GptEntry>> {
    let required_size = header.partition_array_size()?;
    let available_size = u64::try_from(data.len())
        .map_err(|_| AccessorError::volume("GPT entry array length exceeds u64::MAX"))?;

    if available_size < required_size {
        return Err(AccessorError::volume(format!(
            "GPT partition entry array is truncated: expected {required_size} bytes, got {available_size}"
        )));
    }

    let mut input = data;
    let mut entries = Vec::new();

    for slot in 0..header.partition_entry_count {
        let (remaining, entry_data) = nom_take(
            input,
            header.partition_entry_size,
            &format!("GPT partition entry {slot} is truncated"),
        )?;

        input = remaining;

        let entry = parse_gpt_entry(slot, entry_data)?;
        if entry.partition_type_guid != Uuid::nil() {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn parse_gpt_entry(slot: u32, data: &[u8]) -> AccessorResult<GptEntry> {
    let (input, partition_type_guid) = nom_guid(data, "GPT partition type GUID is truncated")?;
    let (input, partition_guid) = nom_guid(input, "GPT unique partition GUID is truncated")?;
    let (input, start_lba) = nom_u64(input, "GPT partition start LBA is truncated")?;
    let (input, end_lba) = nom_u64(input, "GPT partition end LBA is truncated")?;

    let (input, attributes) = nom_u64(input, "GPT partition attributes are truncated")?;
    let (_, name_bytes) = nom_take(input, 72 as u8, "GPT partition name is truncated")?;
    let partition_name = extract_utf16_string(name_bytes);

    if partition_type_guid != Uuid::nil() && end_lba < start_lba {
        return Err(AccessorError::volume(format!(
            "GPT partition entry {slot} has invalid LBA range {start_lba}-{end_lba}"
        )));
    }

    Ok(GptEntry {
        slot,
        partition_type_guid,
        partition_guid,
        start_lba,
        end_lba,
        attributes,
        partition_name,
    })
}

/// Mappings of popular GUID partitions. From: <https://en.wikipedia.org/wiki/GUID_Partition_Table>
/// CSV under tools folder contains list
fn guid_mapping() -> HashMap<String, GuidNames> {
    HashMap::from([
        (
            String::from("05816CE2-DD40-4AC6-A61D-37D32DC1BA7D"),
            GuidNames::Linux,
        ),
        (
            String::from("0657FD6D-A4AB-43C4-84E5-0933C84B4F4F"),
            GuidNames::Swap,
        ),
        (
            String::from("08A7ACEA-624C-4A20-91E8-6E0FA67D23F9"),
            GuidNames::Linux,
        ),
        (
            String::from("0B888863-D7F8-4D9E-9766-239FCE4D58AF"),
            GuidNames::Linux,
        ),
        (
            String::from("0F4868E9-9952-4706-979F-3ED3A473E947"),
            GuidNames::Linux,
        ),
        (
            String::from("0FC63DAF-8483-4772-8E79-3D69D8477DE4"),
            GuidNames::Linux,
        ),
        (
            String::from("143A70BA-CBD3-4F06-919F-6C05683A78BC"),
            GuidNames::Linux,
        ),
        (
            String::from("15BB03AF-77E7-4D4A-B12B-C0D084F7491C"),
            GuidNames::Linux,
        ),
        (
            String::from("15DE6170-65D3-431C-916E-B0DCD8393F25"),
            GuidNames::Linux,
        ),
        (
            String::from("16B417F8-3E06-4F57-8DD2-9B5232F41AA6"),
            GuidNames::Linux,
        ),
        (
            String::from("17440E4F-A8D0-467F-A46E-3912AE6EF2C5"),
            GuidNames::Linux,
        ),
        (
            String::from("1AACDB3B-5444-4138-BD9E-E5C2239B2346"),
            GuidNames::Linux,
        ),
        (
            String::from("1B31B5AA-ADD9-463A-B2ED-BD467FC857E7"),
            GuidNames::Linux,
        ),
        (
            String::from("1DE3F1EF-FA98-47B5-8DCD-4A860A654D78"),
            GuidNames::Linux,
        ),
        (
            String::from("24B2D975-0F97-4521-AFA1-CD531E421B8D"),
            GuidNames::Linux,
        ),
        (
            String::from("2C7357ED-EBD2-46D9-AEC1-23D437EC2BF5"),
            GuidNames::Linux,
        ),
        (
            String::from("2C9739E2-F068-46B3-9FD0-01C5A9AFBCCA"),
            GuidNames::Linux,
        ),
        (
            String::from("2FB4BF56-07FA-42DA-8132-6B139F2026AE"),
            GuidNames::Linux,
        ),
        (
            String::from("31741CC4-1A2A-4111-A581-E00B447D2D06"),
            GuidNames::Linux,
        ),
        (
            String::from("3482388E-4254-435A-A241-766A065F9960"),
            GuidNames::Linux,
        ),
        (
            String::from("37C58C8A-D913-4156-A25F-48B1B64E07F0"),
            GuidNames::Linux,
        ),
        (
            String::from("3A112A75-8729-4380-B4CF-764D79934448"),
            GuidNames::Linux,
        ),
        (
            String::from("3B8F8425-20E0-4F3B-907F-1A25A76F98E8"),
            GuidNames::Linux,
        ),
        (
            String::from("3C3D61FE-B5F3-414D-BB71-8739A694A4EF"),
            GuidNames::Linux,
        ),
        (
            String::from("3E23CA0B-A4BC-4B4E-8087-5AB6A26AA8A9"),
            GuidNames::Linux,
        ),
        (
            String::from("3F324816-667B-46AE-86EE-9B0C0C6C11B4"),
            GuidNames::Linux,
        ),
        (
            String::from("41092B05-9FC8-4523-994F-2DEF0408B176"),
            GuidNames::Linux,
        ),
        (
            String::from("42B0455F-EB11-491D-98D3-56145BA9D037"),
            GuidNames::Linux,
        ),
        (
            String::from("4301D2A6-4E3B-4B2A-BB94-9E0B2C4225EA"),
            GuidNames::Linux,
        ),
        (
            String::from("43CE94D4-0F3D-4999-8250-B9DEAFD98E6E"),
            GuidNames::Linux,
        ),
        (
            String::from("44479540-F297-41B2-9AF7-D131D5F0458A"),
            GuidNames::Linux,
        ),
        (
            String::from("450DD7D1-3224-45EC-9CF2-A43A346D71EE"),
            GuidNames::Linux,
        ),
        (
            String::from("46B98D8D-B55C-4E8F-AAB3-37FCA7F80752"),
            GuidNames::Linux,
        ),
        (
            String::from("4EDE75E2-6CCC-4CC8-B9C7-70334B087510"),
            GuidNames::Linux,
        ),
        (
            String::from("4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709"),
            GuidNames::Linux,
        ),
        (
            String::from("55497029-C7C1-44CC-AA39-815ED1558630"),
            GuidNames::Linux,
        ),
        (
            String::from("579536F8-6A33-4055-A95A-DF2D5E2C42A8"),
            GuidNames::Linux,
        ),
        (
            String::from("57E13958-7331-4365-8E6E-35EEEE17C61B"),
            GuidNames::Linux,
        ),
        (
            String::from("5843D618-EC37-48D7-9F12-CEA8E08768B2"),
            GuidNames::Linux,
        ),
        (
            String::from("5996FC05-109C-48DE-808B-23FA0830B676"),
            GuidNames::Linux,
        ),
        (
            String::from("5AFB67EB-ECC8-4F85-AE8E-AC1E7C50E7D0"),
            GuidNames::Linux,
        ),
        (
            String::from("5C6E1C76-076A-457A-A0FE-F3B4CD21CE6E"),
            GuidNames::Linux,
        ),
        (
            String::from("5EEAD9A9-FE09-4A1E-A1D7-520D00531306"),
            GuidNames::Linux,
        ),
        (
            String::from("60D5A7FE-8E7D-435C-B714-3DD8162144E1"),
            GuidNames::Linux,
        ),
        (
            String::from("6523F8AE-3EB1-4E2A-A05A-18B695AE656F"),
            GuidNames::Linux,
        ),
        (
            String::from("69DAD710-2CE4-4E3C-B16C-21A1D49ABED3"),
            GuidNames::Linux,
        ),
        (
            String::from("6A491E03-3BE7-4545-8E38-83320E0EA880"),
            GuidNames::Linux,
        ),
        (
            String::from("6DB69DE6-29F4-4758-A7A5-962190F00CE3"),
            GuidNames::Linux,
        ),
        (
            String::from("6E11A4E7-FBCA-4DED-B9E9-E1A512BB664E"),
            GuidNames::Linux,
        ),
        (
            String::from("6E5A1BC8-D223-49B7-BCA8-37A5FCCEB996"),
            GuidNames::Linux,
        ),
        (
            String::from("7007891D-D371-4A80-86A4-5CB875B9302E"),
            GuidNames::Linux,
        ),
        (
            String::from("700BDA43-7A34-4507-B179-EEB93D7A7CA3"),
            GuidNames::Linux,
        ),
        (
            String::from("72EC70A6-CF74-40E6-BD49-4BDA08E8F224"),
            GuidNames::Linux,
        ),
        (
            String::from("7386CDF2-203C-47A9-A498-F2ECCE45A2D6"),
            GuidNames::Linux,
        ),
        (
            String::from("75250D76-8CC6-458E-BD66-BD47CC81A812"),
            GuidNames::Linux,
        ),
        (
            String::from("77055800-792C-4F94-B39A-98C91B762BB6"),
            GuidNames::Linux,
        ),
        (
            String::from("773B2ABC-2A99-4398-8BF5-03BAAC40D02B"),
            GuidNames::Linux,
        ),
        (
            String::from("773F91EF-66D4-49B5-BD83-D683BF40AD16"),
            GuidNames::Linux,
        ),
        (
            String::from("77FF5F63-E7B6-4633-ACF4-1565B864C0E6"),
            GuidNames::Linux,
        ),
        (
            String::from("7978A683-6316-4922-BBEE-38BFF5A2FECC"),
            GuidNames::Linux,
        ),
        (
            String::from("7A430799-F711-4C7E-8E5B-1D685BD48607"),
            GuidNames::Linux,
        ),
        (
            String::from("7AC63B47-B25C-463B-8DF8-B4A94E6C90E1"),
            GuidNames::Linux,
        ),
        (
            String::from("7D0359A3-02B3-4F0A-865C-654403E70625"),
            GuidNames::Linux,
        ),
        (
            String::from("7D14FEC5-CC71-415D-9D6C-06BF0B3C3EAF"),
            GuidNames::Linux,
        ),
        (
            String::from("7FFEC5C9-2D00-49B7-8941-3EA10A5586B7"),
            GuidNames::Linux,
        ),
        (
            String::from("81CF9D90-7458-4DF4-8DCF-C8A3A404F09B"),
            GuidNames::Linux,
        ),
        (
            String::from("8484680C-9521-48C6-9C11-B0720656F69E"),
            GuidNames::Linux,
        ),
        (
            String::from("86ED10D5-B607-45BB-8957-D350F23D0571"),
            GuidNames::Linux,
        ),
        (
            String::from("8A4F5770-50AA-4ED3-874A-99B710DB6FEA"),
            GuidNames::Linux,
        ),
        (
            String::from("8CCE0D25-C0D0-4A44-BD87-46331BF1DF67"),
            GuidNames::Linux,
        ),
        (
            String::from("8DA63339-0007-60C0-C436-083AC8230908"),
            GuidNames::Linux,
        ),
        (
            String::from("8DE58BC2-2A43-460D-B14E-A76E4A17B47F"),
            GuidNames::Linux,
        ),
        (
            String::from("8F1056BE-9B05-47C4-81D6-BE53128E5B54"),
            GuidNames::Linux,
        ),
        (
            String::from("8F461B0D-14EE-4E81-9AA9-049B6FB97ABD"),
            GuidNames::Linux,
        ),
        (
            String::from("904E58EF-5C65-4A31-9C57-6AF5FC7C5DE7"),
            GuidNames::Linux,
        ),
        (
            String::from("906BD944-4589-4AAE-A4E4-DD983917446A"),
            GuidNames::Linux,
        ),
        (
            String::from("912ADE1D-A839-4913-8964-A10EEE08FBD2"),
            GuidNames::Linux,
        ),
        (
            String::from("9225A9A3-3C19-4D89-B4F6-EEFF88F17631"),
            GuidNames::Linux,
        ),
        (
            String::from("933AC7E1-2EB4-4F13-B844-0E14E2AEF915"),
            GuidNames::Linux,
        ),
        (
            String::from("94F9A9A1-9971-427A-A400-50CB297F0F35"),
            GuidNames::Linux,
        ),
        (
            String::from("966061EC-28E4-4B2E-B4A5-1F0A825A1D84"),
            GuidNames::Linux,
        ),
        (
            String::from("974A71C0-DE41-43C3-BE5D-5C5CCD1AD2C0"),
            GuidNames::Linux,
        ),
        (
            String::from("97AE158D-F216-497B-8057-F7F905770F54"),
            GuidNames::Linux,
        ),
        (
            String::from("98CFE649-1588-46DC-B2F0-ADD147424925"),
            GuidNames::Linux,
        ),
        (
            String::from("993D8D3D-F80E-4225-855A-9DAF8ED7EA97"),
            GuidNames::Linux,
        ),
        (
            String::from("A19D880F-05FC-4D3B-A006-743F0F84911E"),
            GuidNames::Linux,
        ),
        (
            String::from("AE0253BE-1167-4007-AC68-43926C14C5DE"),
            GuidNames::Linux,
        ),
        (
            String::from("B024F315-D330-444C-8461-44BBDE524E99"),
            GuidNames::Linux,
        ),
        (
            String::from("B0E01050-EE5F-4390-949A-9101B17104E9"),
            GuidNames::Linux,
        ),
        (
            String::from("B325BFBE-C7BE-4AB8-8357-139E652D2F6B"),
            GuidNames::Linux,
        ),
        (
            String::from("B3671439-97B0-4A53-90F7-2D5A8F3AD47B"),
            GuidNames::Linux,
        ),
        (
            String::from("B663C618-E7BC-4D6D-90AA-11B756BB1797"),
            GuidNames::Linux,
        ),
        (
            String::from("B6ED5582-440B-4209-B8DA-5FF7C419EA3D"),
            GuidNames::Linux,
        ),
        (
            String::from("B921B045-1DF0-41C3-AF44-4C6F280D3FAE"),
            GuidNames::Linux,
        ),
        (
            String::from("B933FB22-5C3F-4F91-AF90-E2BB0FA50702"),
            GuidNames::Linux,
        ),
        (
            String::from("BBA210A2-9C5D-45EE-9E87-FF2CCBD002D0"),
            GuidNames::Linux,
        ),
        (
            String::from("BC13C2FF-59E6-4262-A352-B275FD6F7172"),
            GuidNames::Linux,
        ),
        (
            String::from("BDB528A5-A259-475F-A87D-DA53FA736A07"),
            GuidNames::Linux,
        ),
        (
            String::from("BEAEC34B-8442-439B-A40B-984381ED097D"),
            GuidNames::Linux,
        ),
        (
            String::from("C215D751-7BCD-4649-BE90-6627490A4C05"),
            GuidNames::Linux,
        ),
        (
            String::from("C23CE4FF-44BD-4B00-B2D4-B41B3419E02A"),
            GuidNames::Linux,
        ),
        (
            String::from("C31C45E6-3F39-412E-80FB-4809C4980599"),
            GuidNames::Linux,
        ),
        (
            String::from("C3836A13-3137-45BA-B583-B16C50FE5EB4"),
            GuidNames::Linux,
        ),
        (
            String::from("C50CDD70-3862-4CC3-90E1-809A8C93EE2C"),
            GuidNames::Linux,
        ),
        (
            String::from("C80187A5-73A3-491A-901A-017C3FA953E9"),
            GuidNames::Linux,
        ),
        (
            String::from("C8BFBD1E-268E-4521-8BBA-BF314C399557"),
            GuidNames::Linux,
        ),
        (
            String::from("C919CC1F-4456-4EFF-918C-F75E94525CA5"),
            GuidNames::Linux,
        ),
        (
            String::from("C97C1F32-BA06-40B4-9F22-236061B08AA8"),
            GuidNames::Linux,
        ),
        (
            String::from("CA7D7CCB-63ED-4C53-861C-1742536059CC"),
            GuidNames::Linux,
        ),
        (
            String::from("CB1EE4E3-8CD0-4136-A0A4-AA61A32E8730"),
            GuidNames::Linux,
        ),
        (
            String::from("CD0F869B-D0FB-4CA0-B141-9EA87CC78D66"),
            GuidNames::Linux,
        ),
        (
            String::from("D113AF76-80EF-41B4-BDB6-0CFF4D3D4A25"),
            GuidNames::Linux,
        ),
        (
            String::from("D13C5D3B-B5D1-422A-B29F-9454FDC89D76"),
            GuidNames::Linux,
        ),
        (
            String::from("D212A430-FBC5-49F9-A983-A7FEEF2B8D0E"),
            GuidNames::Linux,
        ),
        (
            String::from("D27F46ED-2919-4CB8-BD25-9531F3C16534"),
            GuidNames::Linux,
        ),
        (
            String::from("D2F9000A-7A18-453F-B5CD-4D32F77A7B32"),
            GuidNames::Linux,
        ),
        (
            String::from("D46495B7-A053-414F-80F7-700C99921EF8"),
            GuidNames::Linux,
        ),
        (
            String::from("D4A236E7-E873-4C07-BF1D-BF6CF7F1C3C6"),
            GuidNames::Linux,
        ),
        (
            String::from("D7D150D2-2A04-4A33-8F12-16651205FF7B"),
            GuidNames::Linux,
        ),
        (
            String::from("D7FF812F-37D1-4902-A810-D76BA57B975A"),
            GuidNames::Linux,
        ),
        (
            String::from("DC4A4480-6917-4262-A4EC-DB9384949F25"),
            GuidNames::Linux,
        ),
        (
            String::from("DF3300CE-D69F-4C92-978C-9BFB0F38D820"),
            GuidNames::Linux,
        ),
        (
            String::from("DF765D00-270E-49E5-BC75-F47BB2118B09"),
            GuidNames::Linux,
        ),
        (
            String::from("E18CF08C-33EC-4C0D-8246-C6C6FB3DA024"),
            GuidNames::Linux,
        ),
        (
            String::from("E611C702-575C-4CBE-9A46-434FA0BF7E3F"),
            GuidNames::Linux,
        ),
        (
            String::from("E6D6D379-F507-44C2-A23C-238F2A3DF928"),
            GuidNames::Linux,
        ),
        (
            String::from("E7BB33FB-06CF-4E81-8273-E543B413E2E2"),
            GuidNames::Linux,
        ),
        (
            String::from("E9434544-6E2C-47CC-BAE2-12D6DEAFB44C"),
            GuidNames::Linux,
        ),
        (
            String::from("E98B36EE-32BA-4882-9B12-0CE14655F46A"),
            GuidNames::Linux,
        ),
        (
            String::from("EE2B9983-21E8-4153-86D9-B6901A54D1CE"),
            GuidNames::Linux,
        ),
        (
            String::from("EFE0F087-EA8D-4469-821A-4C2A96A8386A"),
            GuidNames::Linux,
        ),
        (
            String::from("F2C2C7EE-ADCC-4351-B5C6-EE9816B66E16"),
            GuidNames::Linux,
        ),
        (
            String::from("F3393B22-E9AF-4613-A948-9D3BFBD0C535"),
            GuidNames::Linux,
        ),
        (
            String::from("F46B2C26-59AE-48F0-9106-C50ED47F673D"),
            GuidNames::Linux,
        ),
        (
            String::from("F5E2C20C-45B2-4FFA-BCE9-2A60737E1AAF"),
            GuidNames::Linux,
        ),
        (
            String::from("FC56D9E9-E6E5-4C06-BE32-E74407CE09A5"),
            GuidNames::Linux,
        ),
        (
            String::from("FCA0598C-D880-4591-8C16-4EDA05C7347C"),
            GuidNames::Linux,
        ),
        (
            String::from("426F6F74-0000-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("48465300-0000-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("4C616265-6C00-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("52414944-0000-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("52414944-5F4F-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("52637672-7900-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("5265636F-7665-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("53746F72-6167-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("55465300-0000-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("69646961-6700-11AA-AA11-00306543ECAC"),
            GuidNames::Apple,
        ),
        (
            String::from("6A898CC3-1DD2-11B2-99A6-080020736631"),
            GuidNames::Apple,
        ),
        (
            String::from("37AFFC90-EF7D-4E96-91C3-2D7AE055B174"),
            GuidNames::Windows,
        ),
        (
            String::from("558D43C5-A1AC-43C0-AAC8-D1472B2923D1"),
            GuidNames::Windows,
        ),
        (
            String::from("5808C8AA-7E8F-42E0-85D2-E1E90434CFB3"),
            GuidNames::Windows,
        ),
        (
            String::from("AF9B60A0-1431-4F62-BC68-3311714A69AD"),
            GuidNames::Windows,
        ),
        (
            String::from("DE94BBA4-06D1-4D40-A16A-BFD50179D6AC"),
            GuidNames::Windows,
        ),
        (
            String::from("E3C9E316-0B5C-4DB8-817D-F92DF00215AE"),
            GuidNames::Windows,
        ),
        (
            String::from("E75CAF8F-F680-4CEE-AFA3-B001E56EFC2D"),
            GuidNames::Windows,
        ),
        (
            String::from("EBD0A0A2-B9E5-4433-87C0-68B6B72699C7"),
            GuidNames::Windows,
        ),
        (
            String::from("6A82CB45-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A85CF4D-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A87C46F-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A898CC3-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A8B642B-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A8D2AC7-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A8EF2E9-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A90BA39-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A9283A5-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A945A3B-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A96237F-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A9630D1-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("6A980767-1DD2-11B2-99A6-080020736631"),
            GuidNames::Illumos,
        ),
        (
            String::from("00000000-0000-0000-0000-000000000000"),
            GuidNames::Unused,
        ),
        (
            String::from("024DEE41-33E7-11D3-9D69-0008C781F39F"),
            GuidNames::Mbr,
        ),
        (
            String::from("21686148-6449-6E6F-744E-656564454649"),
            GuidNames::Bios,
        ),
        (
            String::from("C12A7328-F81F-11D2-BA4B-00A0C93EC93B"),
            GuidNames::Efi,
        ),
        (
            String::from("516E7CB4-6ECF-11D6-8FF8-00022D09712B"),
            GuidNames::Freebsd,
        ),
        (
            String::from("516E7CB5-6ECF-11D6-8FF8-00022D09712B"),
            GuidNames::Freebsd,
        ),
        (
            String::from("516E7CB6-6ECF-11D6-8FF8-00022D09712B"),
            GuidNames::Freebsd,
        ),
        (
            String::from("516E7CB8-6ECF-11D6-8FF8-00022D09712B"),
            GuidNames::Freebsd,
        ),
        (
            String::from("516E7CBA-6ECF-11D6-8FF8-00022D09712B"),
            GuidNames::Freebsd,
        ),
        (
            String::from("74BA7DD9-A689-11E1-BD04-00E081286ACF"),
            GuidNames::Freebsd,
        ),
        (
            String::from("83BD6B9D-7F41-11DC-BE0B-001560B84F0F"),
            GuidNames::Freebsd,
        ),
        (
            String::from("824CC7A0-36A8-11E3-890A-952519AD3F61"),
            GuidNames::OpenBsd,
        ),
        (
            String::from("9198EFFC-31C0-11DB-8F78-000C2911D1B8"),
            GuidNames::Vmware,
        ),
        (
            String::from("9D275380-40AD-11DB-BF97-000C2911D1B8"),
            GuidNames::Vmware,
        ),
        (
            String::from("AA31E02A-400F-11DB-9590-000C2911D1B8"),
            GuidNames::Vmware,
        ),
        (
            String::from("2DB519C4-B10F-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("2DB519EC-B10F-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("49F48D32-B10E-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("49F48D5A-B10E-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("49F48D82-B10E-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("49F48DAA-B10E-11DC-B99B-0019D1879648"),
            GuidNames::Netbsd,
        ),
        (
            String::from("481B2A38-0561-420B-B72A-F1C4988EFC16"),
            GuidNames::Minix,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use crate::accessor::bootsector::gpt::{
        GuidNames, guid_mapping, parse_gpt_entry, parse_gpt_header,
    };
    use std::{fs::read, path::PathBuf};

    #[test]
    fn test_parse_gpt_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/bootsector/gpt/lba1.raw");
        let bytes = read(test_location.to_str().unwrap()).unwrap();

        let results = parse_gpt_header(bytes.get(512..).unwrap()).unwrap();
        assert_eq!(results.current_lba, 1);
        assert_eq!(results.partition_entry_lba, 2);
        assert_eq!(results.partition_entry_size, 128);
    }

    #[test]
    fn test_parse_gpt_bios_entry() {
        let test = [
            72, 97, 104, 33, 73, 100, 111, 110, 116, 78, 101, 101, 100, 69, 70, 73, 78, 100, 243,
            223, 191, 211, 6, 70, 150, 50, 124, 147, 165, 176, 161, 30, 0, 8, 0, 0, 0, 0, 0, 0,
            255, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];

        let result = parse_gpt_entry(0, &test).unwrap();
        assert_eq!(
            result.partition_type_guid.as_hyphenated().to_string(),
            "21686148-6449-6e6f-744e-656564454649"
        );
        assert_eq!(result.start_lba, 2048);
        assert_eq!(result.end_lba, 4095);
        assert_eq!(
            result.partition_guid.as_hyphenated().to_string(),
            "dff3644e-d3bf-4606-9632-7c93a5b0a11e"
        );
    }

    #[test]
    fn test_parse_gpt_linux_filesystem_entry() {
        let test = [
            175, 61, 198, 15, 131, 132, 114, 71, 142, 121, 61, 105, 216, 71, 125, 228, 162, 39,
            157, 128, 20, 7, 129, 73, 166, 99, 60, 28, 235, 140, 229, 23, 0, 16, 0, 0, 0, 0, 0, 0,
            255, 247, 127, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        let result = parse_gpt_entry(0, &test).unwrap();
        assert_eq!(
            result.partition_type_guid.as_hyphenated().to_string(),
            "0fc63daf-8483-4772-8e79-3d69d8477de4"
        );
        assert_eq!(result.start_lba, 4096);
        assert_eq!(result.end_lba, 209713151);
        assert_eq!(
            result.partition_guid.as_hyphenated().to_string(),
            "809d27a2-0714-4981-a663-3c1ceb8ce517"
        );
    }

    #[test]
    fn test_guid_mappings() {
        let maps = guid_mapping();
        assert_eq!(
            maps.get("41092B05-9FC8-4523-994F-2DEF0408B176").unwrap(),
            &GuidNames::Linux
        );
    }
}

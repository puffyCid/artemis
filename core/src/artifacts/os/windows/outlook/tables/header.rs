use log::warn;

use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

pub(crate) struct TableHeader {
    block_index_offset: u16,
    sig: u8,
    table_type: TableType,
    value_reference: u32,
    fill: Vec<FillLevel>,
}

#[derive(PartialEq, Debug)]
enum TableType {
    SixC,
    SevenC,
    EightC,
    NineC,
    A5,
    Ac,
    B5,
    Bc,
    Cc,
    Unknown,
}

#[derive(PartialEq, Debug)]
enum FillLevel {
    Empty,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9,
    Level10,
    Level11,
    Level12,
    Level13,
    Level14,
    LevelFull,
}

pub(crate) fn table_header(data: &[u8]) -> nom::IResult<&[u8], TableHeader> {
    let (input, block_index_offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (input, sig) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, table_type) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, value_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, level_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let table = TableHeader {
        block_index_offset,
        sig,
        table_type: get_table_type(&table_type),
        value_reference,
        fill: Vec::new(),
    };

    Ok((input, table))
}

fn get_table_type(table: &u8) -> TableType {
    match table {
        0x6c => TableType::SixC,
        0x7c => TableType::SevenC,
        0x8c => TableType::EightC,
        0x9c => TableType::NineC,
        0xa5 => TableType::A5,
        0xac => TableType::Ac,
        0xb5 => TableType::B5,
        0xbc => TableType::Bc,
        0xcc => TableType::Cc,
        _ => {
            warn!("[outlook] Unknown table type: {table}");
            TableType::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::table_header;
    use crate::artifacts::os::windows::outlook::tables::header::TableType;

    #[test]
    fn test_table_header() {
        let test = [162, 0, 236, 124, 64, 0, 0, 0, 0, 0, 0, 0];
        let (_, header) = table_header(&test).unwrap();
        assert_eq!(header.block_index_offset, 162);
        assert_eq!(header.sig, 236);
        assert_eq!(header.table_type, TableType::SevenC);
        assert_eq!(header.value_reference, 64);
        assert_eq!(header.fill, Vec::new());
    }
}

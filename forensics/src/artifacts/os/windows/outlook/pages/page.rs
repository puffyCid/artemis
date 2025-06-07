use crate::utils::nom_helper::{Endian, nom_unsigned_one_byte};
use log::warn;

#[derive(PartialEq, Debug)]
pub(crate) enum PageType {
    /**Also referred as (file)/Offset */
    BlockBtree,
    /**Also referred as (item)/Descriptor */
    NodeBtree,
    FreeMap,
    PageAllocationTable,
    DataAllocationTable,
    FreePage,
    DensityList,
    Unknown,
}

/// Determine page type
pub(crate) fn page_type(data: &[u8]) -> nom::IResult<&[u8], PageType> {
    let (_, page_value) = nom_unsigned_one_byte(data, Endian::Le)?;
    let value = match page_value {
        0x80 => PageType::BlockBtree,
        0x81 => PageType::NodeBtree,
        0x82 => PageType::FreeMap,
        0x83 => PageType::PageAllocationTable,
        0x84 => PageType::DataAllocationTable,
        0x85 => PageType::FreePage,
        0x86 => PageType::DensityList,
        _ => {
            warn!("[outlook] Unknown PageType: {page_value}");
            PageType::Unknown
        }
    };

    Ok((data, value))
}

#[cfg(test)]
mod tests {
    use super::page_type;
    use crate::artifacts::os::windows::outlook::pages::page::PageType;

    #[test]
    fn test_page_type() {
        let test = [134];
        let (_, result) = page_type(&test).unwrap();
        assert_eq!(result, PageType::DensityList);
    }
}

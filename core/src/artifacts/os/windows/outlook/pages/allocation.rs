use super::page::{page_type, PageType};
use crate::{
    artifacts::os::windows::outlook::header::FormatType,
    utils::nom_helper::{nom_unsigned_four_bytes, Endian},
};
use nom::{bytes::complete::take, error::ErrorKind};

pub(crate) struct AllocationTable {
    data: Vec<u8>,
    page_type: PageType,
}
pub(crate) fn parse_allocation_page<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], AllocationTable> {
    let (size, adjust) = match format {
        FormatType::ANSI32 => (512, 12),
        FormatType::Unicode64 => (512, 16),
        FormatType::Unicode64_4k => (4096, 24),
        FormatType::Unknown => {
            // We should never get here
            return Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )));
        }
    };

    let mut input = data;
    if format == &FormatType::ANSI32 {
        let (remaining, _) = nom_unsigned_four_bytes(input, Endian::Le)?;
        input = remaining;
    }
    let (input, values) = take((size - adjust) as u32)(input)?;

    let (_, page_type) = page_type(input)?;
    // Don't care about the rest of the allocation table data

    let allocation = AllocationTable {
        data: values.to_vec(),
        page_type,
    };

    Ok((input, allocation))
}

#[cfg(test)]
mod tests {
    use super::parse_allocation_page;
    use crate::{
        artifacts::os::windows::outlook::{header::FormatType, pages::page::PageType},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_allocation_page() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/allocation.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_allocation_page(&data, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.data.len(), 4072);
        assert_eq!(results.page_type, PageType::DataAllocationTable);
    }
}

use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};

pub(crate) struct RootPage {
    _initial_number_pages: u32,
    _parent_father_data_page_number: u32,
    _extent_space: ExtentSpace,
    _space_tree_page_number: u32,
}

#[derive(Debug, PartialEq)]
enum ExtentSpace {
    Single,
    Multiple,
    Unknown,
}

/// Parse the root page data
pub(crate) fn parse_root_page(data: &[u8]) -> nom::IResult<&[u8], RootPage> {
    let (input, _initial_number_pages) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _parent_father_data_page_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, extent) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _space_tree_page_number) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let single = 1;
    let multiple = 2;
    let _extent_space = if extent == single {
        ExtentSpace::Single
    } else if extent == multiple {
        ExtentSpace::Multiple
    } else {
        ExtentSpace::Unknown
    };

    let root_page = RootPage {
        _initial_number_pages,
        _parent_father_data_page_number,
        _extent_space,
        _space_tree_page_number,
    };

    Ok((input, root_page))
}

#[cfg(test)]
mod tests {
    use super::{parse_root_page, ExtentSpace};

    #[test]
    fn test_parse_root_page() {
        let test = [20, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0];
        let (_, results) = parse_root_page(&test).unwrap();
        assert_eq!(results._extent_space, ExtentSpace::Single);
        assert_eq!(results._initial_number_pages, 20);
        assert_eq!(results._parent_father_data_page_number, 1);
        assert_eq!(results._space_tree_page_number, 5);
    }
}

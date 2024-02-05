use super::class::{parse_class, ClassInfo};
use crate::utils::nom_helper::{
    nom_data, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian,
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};

/// Parse Objects.data file
pub(crate) fn parse_objects<'a>(
    data: &'a [u8],
    pages: &[u32],
) -> nom::IResult<&'a [u8], Vec<ObjectPage>> {
    let page_size = 8192;

    let mut objects = Vec::new();
    let mut skip = 0;
    // Loop through all pages from mappings file
    for (index, page) in pages.iter().enumerate() {
        if skip > 0 {
            skip -= 1;
            continue;
        }
        // Skip unused pages
        if page == &0xffffffff {
            continue;
        }
        let (page_start, _) = take(page * page_size)(data)?;
        let (_, page_data) = take(page_size)(page_start)?;

        let (_, (mut object_page, additional_pages)) = parse_page(page_data, data, &index, pages)?;
        // If we consumed additional pages. We skip equal number pages in loop.
        skip = additional_pages;

        objects.append(&mut object_page);
    }
    Ok((data, objects))
}

#[derive(Debug)]
pub(crate) struct ObjectPage {
    pub(crate) record_id: u32,
    _offset: u32,
    _size: u32,
    _checksum: u32,
    pub(crate) object_data: Vec<u8>,
}

/// Parse the object page
fn parse_page<'a>(
    data: &'a [u8],
    object_remaining: &'a [u8],
    index: &usize,
    pages: &[u32],
) -> nom::IResult<&'a [u8], (Vec<ObjectPage>, u32)> {
    let mut input = data;
    let page_remaining = object_remaining;
    let mut objects: Vec<ObjectPage> = Vec::new();

    let mut additional_pages = 0;

    loop {
        let (remaining, record_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, checksum) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let page_size: u32 = 8192;

        // Last entry is 16 bytes of zeros
        if record_id == 0 && offset == 0 && size == 0 && checksum == 0 {
            break;
        }

        let (data_start, _) = take(offset)(data)?;
        let mut object = ObjectPage {
            record_id,
            _offset: offset,
            _size: size,
            _checksum: checksum,
            object_data: Vec::new(),
        };

        if size > page_size {
            // Pages are 8192 bytes. Need to determine real size of data. Ex: If size = 9000 bytes, thats two pages
            let real_size = size as f32 / page_size as f32;

            additional_pages = if real_size.fract() == 0.0 {
                real_size as u32
            } else {
                // Already nom'd first 8192 bytes
                let adjust_page = 1;
                real_size.ceil() as u32 - adjust_page
            };

            let mut get_pages = 1;
            let mut object_page = data_start.to_vec();

            // Since data is too large to fit in one page. We need to grab more pages
            while get_pages <= additional_pages {
                let page_opt = pages.get(index + get_pages as usize);
                let page = if let Some(result) = page_opt {
                    result
                } else {
                    error!("[wmi] Failed to get more pages for large data");
                    break;
                };

                let (data_start, _) = take(page * page_size)(page_remaining)?;
                let (_, large_data) = take(page_size)(data_start)?;

                object_page.append(&mut large_data.to_vec());
                get_pages += 1;
            }

            let data_result = nom_data(&object_page, size as u64);
            match data_result {
                Ok((_, result)) => object.object_data = result.to_vec(),
                Err(_err) => {
                    error!("[wmi] Failed to nom object data");
                    break;
                }
            }
        } else {
            let (_, object_data) = take(size)(data_start)?;
            object.object_data = object_data.to_vec();
        }

        objects.push(object);
        input = remaining;
    }

    Ok((page_remaining, (objects, additional_pages)))
}

/// Parse Object record
pub(crate) fn parse_record(data: &[u8]) -> nom::IResult<&[u8], ClassInfo> {
    let (input, super_class_name_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let adjust_size = 2;

    // If name size too large. Its probably an Instance block
    if (super_class_name_size * adjust_size) as usize > input.len() {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            ErrorKind::Fail,
        )));
    }

    // Name is UTF16 need to double name size length
    let (input, _super_class_name_data) = take(super_class_name_size * adjust_size)(input)?;
    let (input, _created) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, class_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_class_size = 4;
    // Class size includes size itself. We already nom'd that
    let (input, class_data) = take(class_size - adjust_class_size)(input)?;
    let (_, class_info) = parse_class(class_data)?;

    // Remaining input if any is method data. Which is undocumented

    Ok((input, class_info))
}

#[cfg(test)]
mod tests {
    use super::parse_page;
    use crate::{
        artifacts::os::windows::wmi::{
            map::parse_map,
            objects::{parse_objects, parse_record},
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_page() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/object_page.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();

        let mut skip = 0;
        // Loop through all pages from mappings file
        for (index, page) in results.mappings.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            // Skip unused pages
            if page == &0xffffffff {
                continue;
            }

            let (_, (object_page, additional_pages)) =
                parse_page(&data, &object_data, &index, &results.mappings).unwrap();
            assert_eq!(object_page.len(), 22);
            assert_eq!(additional_pages, 0);

            break;
        }
    }

    #[test]
    fn test_parse_objects() {
        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&data).unwrap();

        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let (_, results) = parse_objects(&data, &results.mappings).unwrap();

        assert!(results.len() > 10);
    }

    #[test]
    fn test_parse_record() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/object_record.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_record(&data).unwrap();

        assert_eq!(results.properties.len(), 3);
    }
}

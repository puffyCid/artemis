use super::class::{parse_class, ClassInfo};
use crate::utils::nom_helper::{
    nom_data, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian,
};
use nom::bytes::complete::take;

/// Parse Objects.data file
pub(crate) fn parse_objects(data: &[u8]) -> nom::IResult<&[u8], Vec<ObjectPage>> {
    let page_size = 8192;
    let mut input = data;

    let mut objects = Vec::new();
    while input.len() >= page_size {
        let (remaining, page_data) = take(page_size)(input)?;
        let (remaining, mut object_page) = parse_page(page_data, remaining)?;
        input = remaining;

        objects.append(&mut object_page);
    }
    Ok((data, objects))
}

#[derive(Debug)]
pub(crate) struct ObjectPage {
    pub(crate) record_id: u32,
    offset: u32,
    size: u32,
    checksum: u32,
    pub(crate) object_data: Vec<u8>,
}

/// Parse the object page
fn parse_page<'a>(
    data: &'a [u8],
    object_remaining: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<ObjectPage>> {
    let mut input = data;
    let mut page_remaining = object_remaining;
    let mut objects: Vec<ObjectPage> = Vec::new();

    loop {
        let (remaining, record_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, checksum) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let page_size: u32 = 8192;

        // Last entry is 16 bytes of zeros
        if record_id == 0 && offset == 0 && size == 0 && checksum == 0 {
            break;
        } else if offset as usize > data.len() {
            println!("strange offset is super large??");
            break;
        } else if size as usize > page_remaining.len() && size > page_size {
            println!("size is larger than whole file. strage..?");
            break;
        }

        let (data_start, _) = take(offset)(data)?;
        let mut object = ObjectPage {
            record_id,
            offset,
            size,
            checksum,
            object_data: Vec::new(),
        };

        if size > page_size {
            let (remaining_input, large_data) = take(
                ((size as f32 / page_size as f32).round() as u32) * page_size,
            )(page_remaining)?;

            page_remaining = remaining_input;
            let mut object_page = data_start.to_vec();
            object_page.append(&mut large_data.to_vec());

            let data_result = nom_data(&object_page, size as u64);
            match data_result {
                Ok((_, result)) => object.object_data = result.to_vec(),
                Err(_err) => {
                    panic!("yikes");
                }
            }
        } else {
            let (_, object_data) = take(size)(data_start)?;
            object.object_data = object_data.to_vec();
        }

        objects.push(object);
        input = remaining;
    }

    Ok((page_remaining, objects))
}

pub(crate) fn parse_record(data: &[u8]) -> nom::IResult<&[u8], ClassInfo> {
    let (input, super_class_name_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let adjust_size = 2;

    // Name is UTF16 need to double name size length
    let (input, super_class_name_data) = take(super_class_name_size * adjust_size)(input)?;
    let (input, created) = nom_unsigned_eight_bytes(input, Endian::Le)?;
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
        artifacts::os::windows::wmi::objects::{parse_objects, parse_record},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_page_page() {
        let data = [
            32, 24, 11, 111, 112, 1, 0, 0, 89, 0, 0, 0, 0, 0, 0, 0, 202, 7, 219, 93, 201, 1, 0, 0,
            180, 0, 0, 0, 0, 0, 0, 0, 97, 220, 68, 21, 125, 2, 0, 0, 209, 0, 0, 0, 0, 0, 0, 0, 207,
            70, 141, 160, 78, 3, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0, 194, 87, 223, 160, 22, 4, 0, 0,
            148, 7, 0, 0, 0, 0, 0, 0, 216, 234, 98, 43, 170, 11, 0, 0, 203, 0, 0, 0, 0, 0, 0, 0,
            193, 123, 236, 120, 117, 12, 0, 0, 134, 3, 0, 0, 0, 0, 0, 0, 53, 109, 160, 85, 251, 15,
            0, 0, 250, 2, 0, 0, 0, 0, 0, 0, 226, 13, 107, 44, 245, 18, 0, 0, 239, 0, 0, 0, 0, 0, 0,
            0, 248, 239, 19, 134, 228, 19, 0, 0, 69, 1, 0, 0, 0, 0, 0, 0, 231, 55, 155, 52, 41, 21,
            0, 0, 197, 0, 0, 0, 0, 0, 0, 0, 139, 120, 221, 154, 238, 21, 0, 0, 7, 1, 0, 0, 0, 0, 0,
            0, 175, 32, 167, 131, 245, 22, 0, 0, 19, 1, 0, 0, 0, 0, 0, 0, 200, 157, 184, 44, 8, 24,
            0, 0, 154, 0, 0, 0, 0, 0, 0, 0, 207, 85, 128, 99, 162, 24, 0, 0, 126, 1, 0, 0, 0, 0, 0,
            0, 77, 169, 6, 118, 32, 26, 0, 0, 98, 0, 0, 0, 0, 0, 0, 0, 49, 229, 16, 254, 130, 26,
            0, 0, 179, 1, 0, 0, 0, 0, 0, 0, 231, 117, 155, 178, 53, 28, 0, 0, 140, 0, 0, 0, 0, 0,
            0, 0, 250, 14, 24, 202, 193, 28, 0, 0, 31, 1, 0, 0, 0, 0, 0, 0, 211, 169, 210, 142,
            224, 29, 0, 0, 88, 0, 0, 0, 0, 0, 0, 0, 193, 244, 193, 161, 56, 30, 0, 0, 111, 0, 0, 0,
            0, 0, 0, 0, 176, 24, 242, 198, 167, 30, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 106, 90, 152, 37, 223, 172, 213, 1, 65, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 15, 0, 0, 0, 15, 0, 0, 0, 0, 11, 0, 0, 0,
            255, 255, 0, 0, 0, 0, 25, 0, 0, 128, 0, 95, 95, 83, 121, 115, 116, 101, 109, 67, 108,
            97, 115, 115, 0, 0, 97, 98, 115, 116, 114, 97, 99, 116, 0, 12, 0, 0, 0, 0, 0, 67, 108,
            0, 0, 0, 128, 13, 0, 0, 0, 95, 0, 95, 0, 83, 0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0,
            67, 0, 108, 0, 97, 0, 115, 0, 115, 0, 107, 90, 152, 37, 223, 172, 213, 1, 130, 0, 0, 0,
            0, 0, 0, 0, 0, 5, 0, 0, 0, 23, 0, 0, 0, 0, 95, 95, 83, 121, 115, 116, 101, 109, 67,
            108, 97, 115, 115, 0, 15, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 13, 0, 0, 0, 19, 0, 0, 0,
            13, 255, 255, 255, 255, 69, 0, 0, 128, 0, 95, 95, 78, 65, 77, 69, 83, 80, 65, 67, 69,
            0, 0, 78, 97, 109, 101, 0, 8,
        ];
        let (_, results) = parse_page(&data, &data).unwrap();
        assert_eq!(results.len(), 22);
    }

    #[test]
    fn test_parse_objects() {
        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let (_, results) = parse_objects(&data).unwrap();

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

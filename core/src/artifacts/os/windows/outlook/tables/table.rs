use super::{
    header::table_header,
    property::{parse_property_context, PropertyContext},
};

pub(crate) fn parse_table_data(data: &[u8]) {
    let (input, header) = table_header(data).unwrap();
    if header.sig == 236 && header.page_map.allocation_count > 0 {
        println!("The table: {header:?}");
    }
}

pub(crate) fn property_context_table(data: &[u8]) -> nom::IResult<&[u8], Vec<PropertyContext>> {
    let (input, property_table) = parse_property_context(data)?;

    Ok((input, property_table))
}

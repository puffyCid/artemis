use super::header::table_header;

pub(crate) fn parse_table_data(data: &[u8]) {
    let (input, header) = table_header(data).unwrap();
    if header.sig == 236 && header.page_map.allocation_count > 0 {
        println!("The table: {header:?}");
    }
}

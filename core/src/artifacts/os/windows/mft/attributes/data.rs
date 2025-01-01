pub(crate) fn parse_data(data: &[u8]) -> nom::IResult<&[u8], ()> {
    panic!("{data:?}");
    Ok((data, ()))
}

use quick_xml::{Reader, escape::unescape, name::QName};

/// Read task XML text content and unescape entity references after parsing.
pub(crate) fn read_text_unescaped(reader: &mut Reader<&[u8]>, name: QName<'_>) -> String {
    let text = reader.read_text(name).unwrap_or_default();
    unescape(text.as_ref())
        .unwrap_or(std::borrow::Cow::Borrowed(text.as_ref()))
        .into_owned()
}

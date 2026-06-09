use crate::output::{
    context::ArtifactContext,
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::OutputResult,
    record::RecordStream,
};
use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};
use serde_json::{Map, Value};
use std::io::Write;

/// Encoder for XML files
#[derive(Debug, PartialEq)]
pub(crate) struct XmlEncoder;

impl ArtifactEncoder for XmlEncoder {
    fn extension(&self) -> &str {
        "xml"
    }

    fn mime_type(&self) -> &str {
        "application/xml"
    }

    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let mut count = 0;
        let mut xml_writer = Writer::new(writer);

        xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
        start_element(&mut xml_writer, "records")?;
        while let Some(record) = records.next_record()? {
            let mut value = record.into_value()?;
            append_metadata(&mut value, context);
            write_value(&mut xml_writer, "record", &value)?;
            count += 1;
        }

        end_element(&mut xml_writer, "records")?;

        Ok(count)
    }
}

/// Write the starting XML element
fn start_element<W: Write>(writer: &mut Writer<W>, name: &str) -> OutputResult<()> {
    Ok(writer.write_event(Event::Start(BytesStart::new(name)))?)
}

/// Write the ending XML element
fn end_element<W: Write>(writer: &mut Writer<W>, name: &str) -> OutputResult<()> {
    Ok(writer.write_event(Event::End(BytesEnd::new(name)))?)
}

/// Write the XML element value
fn write_value<W: Write>(writer: &mut Writer<W>, name: &str, value: &Value) -> OutputResult<()> {
    let name = sanitize_element_name(name);
    match value {
        Value::Object(fields) => write_object(writer, &name, fields),
        Value::Array(values) => write_array(writer, &name, values),
        Value::String(value) => write_text_element(writer, &name, value),
        Value::Number(value) => write_text_element(writer, &name, &value.to_string()),
        Value::Bool(value) => write_text_element(writer, &name, &value.to_string()),
        Value::Null => write_empty_element(writer, &name),
    }
}

/// Ensure element name meets XML requirements
fn sanitize_element_name(name: &str) -> String {
    let mut element_name = String::new();
    for (index, character) in name.chars().enumerate() {
        if index == 0 && !is_xml_name_start(character) {
            element_name.push('_');
        }
        if is_xml_name_char(character) {
            element_name.push(character);
            continue;
        }

        element_name.push('_');
    }

    if element_name.is_empty() {
        element_name = String::from("field");
    }

    // Element names cannot start with "XML"
    if element_name.to_ascii_lowercase().starts_with("xml") {
        element_name.insert(0, '_');
    }

    element_name
}

/// Ensure XML name starts with approved starting character
fn is_xml_name_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

/// Ensure element character is valid for XML
fn is_xml_name_char(ch: char) -> bool {
    is_xml_name_start(ch) || ch.is_ascii_digit() || ch == '-' || ch == '.'
}

/// Write objects to XML
fn write_object<W: Write>(
    writer: &mut Writer<W>,
    name: &str,
    fields: &Map<String, Value>,
) -> OutputResult<()> {
    start_element(writer, name)?;
    for (key, value) in fields {
        write_value(writer, key, value)?;
    }

    end_element(writer, name)?;

    Ok(())
}

/// Write array to XML
fn write_array<W: Write>(writer: &mut Writer<W>, name: &str, values: &[Value]) -> OutputResult<()> {
    start_element(writer, name)?;

    for value in values {
        write_value(writer, name, value)?;
    }

    end_element(writer, name)?;

    Ok(())
}

/// Write text values to XML
fn write_text_element<W: Write>(
    writer: &mut Writer<W>,
    name: &str,
    value: &str,
) -> OutputResult<()> {
    start_element(writer, name)?;

    writer.write_event(Event::Text(BytesText::new(value)))?;
    end_element(writer, name)?;

    Ok(())
}

/// Write empty XML value
fn write_empty_element<W: Write>(writer: &mut Writer<W>, name: &str) -> OutputResult<()> {
    Ok(writer.write_event(Event::Empty(BytesStart::new(name)))?)
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::PathBuf};

    use serde_json::json;

    use crate::{
        output::{
            context::CollectionContext,
            encoder::{artifact_encoder::ArtifactEncoder, xml::XmlEncoder},
            record::{JsonRecord, Record, SingleRecordStream},
        },
        structs::toml::OutputConfig,
    };

    #[test]
    fn test_xml_encoder() {
        let output = OutputConfig::default();

        let context = CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "files",
            &output.start_time_filter,
            &output.end_time_filter,
        );
        let test = json!({
            "path": "/tmp/one.txt",
            "size": 1234,
            "is_file": true,
            "tags": ["one", "maybe"],
            "nested": {
                "key": "rust"
            }
        });

        let mut writer = Cursor::new(Vec::new());
        let count = XmlEncoder
            .encode(
                &mut SingleRecordStream::new(Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))),
                &mut writer,
                &context,
            )
            .unwrap();

        let xml = String::from_utf8(writer.into_inner()).unwrap();
        assert_eq!(count, 1);
        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(xml.contains("<records>"));
        assert!(xml.contains("<record>"));
        assert!(xml.contains("<path>/tmp/one.txt</path>"));
        assert!(xml.contains("<size>1234</size>"));
        assert!(xml.contains("<is_file>true</is_file>"));
        assert!(xml.contains("<tags><tags>one</tags><tags>maybe</tags></tags>"));
        assert!(xml.contains("<nested><key>rust</key></nested>"));
        assert!(xml.contains("<collection_metadata>"));
        assert!(xml.contains("<artifact_name>files</artifact_name>"));
        assert!(xml.ends_with("</records>"));
    }

    #[test]
    fn test_xml_encoder_escape_text() {
        let output = OutputConfig::default();
        let context = CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "files",
            &output.start_time_filter,
            &output.end_time_filter,
        );
        let value = json!({
            "path": "/tmp/a&b<c>.txt"
        });
        let mut writer = Cursor::new(Vec::new());
        XmlEncoder
            .encode(
                &mut SingleRecordStream::new(Record::Json(JsonRecord::new(
                    value.as_object().unwrap().clone(),
                ))),
                &mut writer,
                &context,
            )
            .unwrap();
        let xml = String::from_utf8(writer.into_inner()).unwrap();
        assert!(xml.contains("<path>/tmp/a&amp;b&lt;c&gt;.txt</path>"));
    }

    #[test]
    fn test_xml_encoder_sanitizes_field_names() {
        let output = OutputConfig::default();
        let context = CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "runtime",
            &output.start_time_filter,
            &output.end_time_filter,
        );
        let value = json!({
            "123field": "starts with number",
            "field name": "contains space",
            "field/name": "contains slash"
        });
        let mut writer = Cursor::new(Vec::new());
        XmlEncoder
            .encode(
                &mut SingleRecordStream::new(Record::Json(JsonRecord::new(
                    value.as_object().unwrap().clone(),
                ))),
                &mut writer,
                &context,
            )
            .unwrap();
        let xml = String::from_utf8(writer.into_inner()).unwrap();
        assert!(xml.contains("<_123field>starts with number</_123field>"));
        assert!(xml.contains("<field_name>contains space</field_name>"));
        assert!(xml.contains("<field_name>contains slash</field_name>"));
    }
}

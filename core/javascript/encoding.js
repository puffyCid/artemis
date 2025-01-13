const { core } = globalThis.Deno;
class Encoding {
    atob = (data) => {
        return core.ops.js_base64_decode(data);
    };
    btoa = (data) => {
        return core.ops.js_base64_encode(data);
    };
    extract_utf8_string = (data) => {
        return core.ops.js_extract_utf8_string(data);
    };
    extract_utf16_string = (data) => {
        return core.ops.js_extract_utf16_string(data);
    };
    bytes_encode = (data) => {
        return core.ops.js_encode_bytes(data);
    };
    read_xml = (data) => {
        return core.ops.js_read_xml(data);
    };
    parse_protobuf = (data) => {
        return core.ops.js_parse_protobuf(data);
    };
    bytes_to_hex_string = (data) => {
        return core.ops.js_bytes_to_hex_string(data);
    };
    bytes_to_le_guid = (data) => {
        return core.ops.js_format_guid_le_bytes(data);
    };
    bytes_to_be_guid = (data) => {
        return core.ops.js_format_guid_be_bytes(data);
    };
    generate_uuid = () => {
        return core.ops.js_generate_uuid();
    };
}
export const encoding = new Encoding();

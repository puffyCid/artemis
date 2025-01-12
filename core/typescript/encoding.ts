//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

/**
 * @class Encoding used to encode and decode data through artemis
 */
class Encoding {
  /**
   * Base64 decode a provided string
   * @param data Base64 encoded string to decode
   * @returns Decoded string as raw bytes
   */
  atob = (data: string) => {
    return core.ops.js_base64_decode(data);
  };
  /**
   * Base64 encode a provided raw bytes
   * @param data Raw bytes to encode
   * @returns Base64 string
   */
  btoa = (data: ArrayBuffer) => {
    return core.ops.js_base64_encode(data);
  };
  /**
   * Attempt to extract a UTF8 string from raw bytes
   * @param data Raw bytes to extract string from
   * @returns An extracted string or empty value
   */
  extract_utf8_string = (data: Uint8Array) => {
    return core.ops.js_extract_utf8_string(data);
  };
  /**
   * Attempt to extract a UTF16 string from raw bytes
   * @param data Raw bytes to extract string from
   * @returns An extracted string or empty value
   */
  extract_utf16_string = (data: Uint8Array) => {
    return core.ops.js_extract_utf16_string(data);
  };
  /**
   * Convert provided string to raw bytes
   * @param data String to convert to bytes
   * @returns Encode string into bytes
   */
  bytes_encode = (data: string) => {
    return core.ops.js_encode_bytes(data);
  };
  /**
   * Read a XML file into a JSON object
   * @param data Path to XML file
   * @returns JSON object representing the XML content
   */
  read_xml = (data: string) => {
    return core.ops.js_read_xml(data);
  };
  /**
   * Parse provided Protobuf bytes
   * @param data Protobuf bytes
   * @returns JSON object representing the extracted Protobuf content
   */
  parse_protobuf = (data: Uint8Array) => {
    return core.ops.js_parse_protobuf(data);
  };
  /**
   * Convert raw bytes to a hex string
   * @param data Raw bytes to convert to Hex string
   * @returns A Hexadecimal string
   */
  bytes_to_hex_string = (data: Uint8Array) => {
    return core.ops.js_bytes_to_hex_string(data);
  };
  /**
   * Convert bytes to LE format GUID. Common on Windows
   * @param data Raw bytes to convert to LE js_format_guid_be_bytes
   * @returns GUID string
   */
  bytes_to_le_guid = (data: Uint8Array) => {
    return core.ops.js_format_guid_le_bytes(data);
  };
  /**
   * Convert bytes to BE GUID. Common on macOS
   * @param data Raw bytes to convert to GE GUID
   * @returns GUID string
   */
  bytes_to_be_guid = (data: Uint8Array) => {
    return core.ops.js_format_guid_be_bytes(data);
  };
  /**
   * Generate a UUID string
   * @returns UUID string
   */
  generate_uuid = () => {
    return core.ops.js_generate_uuid();
  };
}

export const encoding = new Encoding();

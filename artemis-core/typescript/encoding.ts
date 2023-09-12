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
        try {
            return core.ops.js_read_xml(data);
        } catch (err) {
            return err;
        }
    };
    /**
     * Convert raw bytes to a hex string
     * @param data Raw bytes to convert to Hex string
     * @returns A Hexadecimal string
     */
    bytes_to_hex_string = (data: Uint8Array) => {
        return core.ops.js_bytes_to_hex_string(data);
    };
}

export const encoding = new Encoding();

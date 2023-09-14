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
    bytes_encode = (data) => {
        return core.ops.js_encode_bytes(data);
    };
    read_xml = (data) => {
        try {
            return core.ops.js_read_xml(data);
        }
        catch (err) {
            return err;
        }
    };
    bytes_to_hex_string = (data) => {
        return core.ops.js_bytes_to_hex_string(data);
    };
}
export const encoding = new Encoding();

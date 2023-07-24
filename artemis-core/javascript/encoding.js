const { core } = globalThis.Deno;
class Encoding {
    atob = (data) => {
        return core.ops.js_base64_decode(data);
    };
    extract_utf8_string = (data) => {
        return core.ops.js_extract_utf8_string(data);
    };
}
export const encoding = new Encoding();

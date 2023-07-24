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
        return core.ops.js_base64_decode(data)
    }
    /**
     * Attempt to extract a UTF8 string from raw bytes
     * @param data Raw bytes to extract string from
     * @returns An extracted string or empty value
     */
    extract_utf8_string = (data: Uint8Array) => {
        return core.ops.js_extract_utf8_string(data)
    }
}

export const encoding = new Encoding();

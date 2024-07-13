const { core } = globalThis.Deno;
class Decryption {
    decrypt_aes = (key, iv, data) => {
        return core.ops.js_decrypt_aes(key, iv, data);
    };
}
export const decryption = new Decryption();

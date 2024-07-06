//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Decryption {
    /**
     * Function to decrypt AES256 data
     * @param key AES256 Key. Must be 32 bytes in size
     * @param iv Initial Vector bytes
     * @param data Encrypted data
     * @returns Decrypted data
     */
    decrypt_aes = (key: Uint8Array, iv: Uint8Array, data: Uint8Array) => {
        return core.ops.js_decrypt_aes(key, iv, data);
    };
}

export const decryption = new Decryption();

//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

class Compression {
    /**
     * Function to decompress zlib compressed data
     * @param data Raw bytes to decompress
     * @param wbits Bit value to use when decompressing
     * @returns Decompressed data
     */
    decompress_zlib = (data: Uint8Array, wbits: number) => {
        return core.ops.js_decompress_zlib(data, wbits);
    };

    /**
     * Function to decompress gzip data
     * @param data Raw gzip bytes to decompress
     * @returns Decompressed data
     */
    decompress_gzip = (data: Uint8Array) => {
        return core.ops.js_decompress_gzip(data);
    }
}

export const compression = new Compression();

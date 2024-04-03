const { core } = globalThis.Deno;
class Compression {
    decompress_zlib = (data, wbits) => {
        return core.ops.js_decompress_zlib(data, wbits);
    };
}
export const compression = new Compression();

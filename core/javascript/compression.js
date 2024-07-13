const { core } = globalThis.Deno;
class Compression {
    decompress_zlib = (data, wbits) => {
        return core.ops.js_decompress_zlib(data, wbits);
    };
    decompress_gzip = (data) => {
        return core.ops.js_decompress_gzip(data);
    };
}
export const compression = new Compression();

const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
const { SymbolAsyncIterator } = primordials;
class FileSystem {
    readDir = async (path) => {
        const data = await core.ops.js_read_dir(path);
        return data;
    };
    stat = (path) => {
        return core.ops.js_stat(path);
    };
    hash = (path, md5, sha1, sha256) => {
        return core.ops.js_hash_file(path, md5, sha1, sha256);
    };
    readTextFile = (path) => {
        return core.ops.js_read_text_file(path);
    };
}
export const filesystem = new FileSystem();

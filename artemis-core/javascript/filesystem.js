const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
class FileSystem {
    readDir = async (path) => {
        try {
            const data = await core.ops.js_read_dir(path);
            return data;
        }
        catch (err) {
            return err;
        }
    };
    stat = (path) => {
        try {
            return core.ops.js_stat(path);
        }
        catch (err) {
            return err;
        }
    };
    hash = (path, md5, sha1, sha256) => {
        try {
            return core.ops.js_hash_file(path, md5, sha1, sha256);
        }
        catch (err) {
            return err;
        }
    };
    readTextFile = (path) => {
        try {
            return core.ops.js_read_text_file(path);
        }
        catch (err) {
            return err;
        }
    };
    glob = (pattern) => {
        try {
            return core.ops.js_glob(pattern);
        }
        catch (err) {
            return err;
        }
    };
    readFile = (path) => {
        try {
            return core.ops.js_read_file(path);
        }
        catch (err) {
            return err;
        }
    };
}
export const filesystem = new FileSystem();

//@ts-ignore: Deno internals
const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
const { SymbolAsyncIterator } = primordials;

/**
 * Class used to interact with the FileSystem through Rust and Deno
 */
class FileSystem {
    /**
     * Lists all files and directories from provided path
     * @param path Path to read
     * @returns Array of file entries
     */
    readDir = (path: string) => {
        const data = core.ops.js_read_dir(path);
        return {
            async *[SymbolAsyncIterator]() {
                const entry = await data;
                for (let i = 0; i< entry.length; ++i) {
                    yield entry[i];
                }
            }
        }
    }
    /**
     * Return metadata for a single file or directory
     * @param path Path to get metadata
     * @returns Metadata about file or directory
     */
    stat = (path: string) => {
        return core.ops.js_stat(path);
    }
    /**
     * Return hashes for a single file
     * @param path Path to file to hash
     * @param md5 Enable MD5 hashing
     * @param sha1 Enable SHA1 hashing
     * @param sha256 Enable SHA256 hashing
     * @returns Collection of hashes
     */
    hash = (path: string, md5: boolean, sha1: boolean, sha256: boolean) => {
        return core.ops.js_hash_file(path, md5, sha1, sha256)
    }
}
export const filesystem = new FileSystem();
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
    readDir = (path) => {
        const data = core.ops.js_read_dir(path);
        return {
            async *[SymbolAsyncIterator]() {
                const entry = await data;
                for (let i = 0; i < entry.length; ++i) {
                    yield entry[i];
                }
            }
        };
    };
    /**
     * Return metadata for a single file or directory
     * @param path Path to get metadata
     * @returns Metadata about file or directory
     */
    stat = (path) => {
        return core.ops.js_stat(path);
    };
}
export const filesystem = new FileSystem();

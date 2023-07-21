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
        return core.ops.js_read_dir(path);
    }
}
export const filesystem = new FileSystem();
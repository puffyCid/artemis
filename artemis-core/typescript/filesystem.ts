//@ts-ignore: Deno internals
const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;

/**
 * @class FileSystem used to interact with the FileSystem through Rust and Deno
 */
class FileSystem {
  /**
   * Lists all files and directories from provided path
   * @param path Path to read
   * @returns Array of file entries
   */
  readDir = async (path: string) => {
    const data = await core.ops.js_read_dir(path);
    return data;
  };
  /**
   * Return metadata for a single file or directory
   * @param path Path to get metadata
   * @returns Metadata about file or directory
   */
  stat = (path: string) => {
    return core.ops.js_stat(path);
  };
  /**
   * Return hashes for a single file
   * @param path File to hash
   * @param md5 Enable MD5 hashing
   * @param sha1 Enable SHA1 hashing
   * @param sha256 Enable SHA256 hashing
   * @returns Collection of hashes
   */
  hash = (path: string, md5: boolean, sha1: boolean, sha256: boolean) => {
    return core.ops.js_hash_file(path, md5, sha1, sha256);
  };
  /**
   * Read a text file. Currently only files less than 2GB in size can be read
   * @param path Text file to read
   * @returns String containing text of file
   */
  readTextFile = (path: string) => {
    return core.ops.js_read_text_file(path);
  };
  /**
   * Process a glob patter and return paths
   * @param pattern Glob pattern to parse
   * @returns String containing paths parsed from glob
   */
  glob = (pattern: string) => {
    return core.ops.js_glob(pattern);
  };
}
export const filesystem = new FileSystem();

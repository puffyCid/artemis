// ../../../artemis-api/src/utils/error.ts
var ErrorBase = class extends Error {
  name;
  message;
  constructor(name, message) {
    super();
    this.name = name;
    this.message = message;
  }
};

// ../../../artemis-api/src/filesystem/errors.ts
var FileError = class extends ErrorBase {
};

// ../../../artemis-api/src/filesystem/files.ts
function readTextFile(handle) {
  try {
    const result = js_read_text_file(handle.File);
    return result;
  } catch (err) {
    return new FileError(
      "READ_TEXT_FILE",
      `failed to read text file ${JSON.stringify(handle.File)}: ${err}`
    );
  }
}

// ../../../artemis-api/src/system/systeminfo.ts
function platform() {
  const data = js_platform();
  return data;
}

// ../../../artemis-api/src/filesystem/accessor.ts
var Accessor = class {
  accessor;
  /**
   * Create the `Accessor`
   */
  constructor() {
    this.accessor = new JsAccessor();
  }
  /**
   * Read a file using the `Accessor`. You can only read files smaller than 2 GB
   * @param path Path to the file to read
   * @param protocol `AccessorProtocol` to use when reading the file. Default is `AccessorProtocol.Host`
   * @returns File content bytes or `FileError`
   */
  readFile(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.read_file(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read file ${target}: ${err}`);
    }
  }
  /**
   * Read a `FileHandle` using the `Accessor`. You can only read files smaller than 2 GB. 
   *  
   * Reading a `FileHandle` lets artemis read a file directly instead of walking the file system 
   * to find. 
   * 
   * Useful when parsing the NTFS filesystem
   * @param handle `FileHandle` to read
   * @returns File content bytes or `FileError`
   */
  readFileHandle(handle) {
    try {
      const results = this.accessor.read_file_handle(handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read FileHandle '${JSON.stringify(handle.File)}': ${err}`);
    }
  }
  /**
   * Read a directory using the `Accessor`.
   * @param path Path to the directory to read
   * @param protocol `AccessorProtocol` to use when reading the file. Default is `AccessorProtocol.Host`
   * @returns Array of `DirEntry` or `FileError`
   */
  readDir(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.read_dir(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read directory ${target}: ${err}`);
    }
  }
  /**
   * Read a `DirHandle` using the `Accessor`.
   *  
   * Reading a `DirHandle` lets artemis read a directory directly instead of walking the file system 
   * to find. 
   * 
   * Useful when parsing the NTFS filesystem
   * @param handle `DirHandle` to read
   * @returns Array of `DirEntry` or `FileError`
   */
  readDirHandle(handle) {
    try {
      const results = this.accessor.read_dir_handle(handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read DirHandle '${JSON.stringify(handle.Directory)}': ${err}`);
    }
  }
  /**
   * Stat a file using the `Accessor`. 
   * @param path Path to the file to read
   * @param protocol `AccessorProtocol` to use when reading the file. Default is `AccessorProtocol.Host`
   * @returns `EntryMeta` value or `FileError`
   */
  stat(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.stat(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat file ${target}: ${err}`);
    }
  }
  /**
   * Stat a `FileHandle` using the `Accessor`.
   *  
   * Stat a `FileHandle` lets artemis stat a file directly instead of walking the file system 
   * to find. 
   * 
   * Useful when parsing the NTFS filesystem
   * @param handle `FileHandle` to read
   * @returns `EntryMeta` value or `FileError`
   */
  statHandle(handle) {
    try {
      const results = this.accessor.stat_handle(handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read FileHandle '${JSON.stringify(handle.File)}': ${err}`);
    }
  }
  /**
   * Stat a `DirHandle` using the `Accessor`.
   *  
   * Stat a `DirHandle` lets artemis stat a file directly instead of walking the file system 
   * to find. 
   * 
   * Useful when parsing the NTFS filesystem
   * @param handle `DirHandle` to stat
   * @returns `EntryMeta` value or `FileError`
   */
  statDirHandle(handle) {
    try {
      const results = this.accessor.stat_dir_handle(handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat DirHandle '${JSON.stringify(handle.Directory)}': ${err}`);
    }
  }
  /**
   * Glob a provided path pattern using the `Accessor`. 
   * @param path Pattern to glob
   * @param protocol `AccessorProtocol` to use when reading the file. Default is `AccessorProtocol.Host`
   * @returns Array of `GlobMatch` or `FileError`
   */
  globfs(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.globfs(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not glob path ${target}: ${err}`);
    }
  }
  /**
   * Open a file for reading. You can stream a file with this function
   * @param path Path to file to return a initialized `Reader` for
   * @param protocol `AccessorProtocol` to use use for the `Reader`
   * @returns `Reader` or `FileError`
   */
  openReader(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.open_reader(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not open reader for path ${target}: ${err}`);
    }
  }
  /**
   * Open a `FileHandle` for reading. You can stream a file with this function
   * @param handle `FileHandle` to read
   * @returns `Reader` or `FileError`
   */
  openReaderHandle(handle) {
    try {
      const results = this.accessor.open_reader_handle(handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not open reader for handle '${JSON.stringify(handle.File)}': ${err}`);
    }
  }
  /**
   * Open a source based on the `AccessorProtocol`.
   * 
   * This can be used to open a source for multiple actions.
   * 
   * For example, opening a NTFS live volume, globbing for files, and then reading them
   * @param path Path to accessor source
   * @param protocol `AccessorProtocol` to use for the source. Default is `AccessorProtocol.Host`
   * @returns `SourceHandle` or `FileError`
   */
  openSource(path, protocol = 0 /* Host */) {
    const target = this.setTarget(path, protocol);
    try {
      const results = this.accessor.open_source(target);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not open source for path ${target}: ${err}`);
    }
  }
  /**
   * Read a file from an open source
   * @param source `SourceHandle` object
   * @param path Path to file to read
   * @returns `Uint8Array` or `FileError`
   */
  sourceReadFile(source, path) {
    try {
      const results = this.accessor.source_read_file(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read file ${path} with source handle: ${err}`);
    }
  }
  /**
   * Read a file from an open source by `FileHandle`
   * @param source `SourceHandle` object
   * @param handle `FileHandle` object
   * @returns `Uint8Array` or `FileError`
   */
  sourceReadFileHandle(source, handle) {
    try {
      const results = this.accessor.source_read_file_handle(source, handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read FIleHandle ${JSON.stringify(handle.File)} with source handle: ${err}`);
    }
  }
  /**
   * Read a directory from an open source
   * @param source `SourceHandle` object
   * @param path Path to file to read
   * @returns Array of `DirEntry` or `FileError`
   */
  sourceReadDir(source, path) {
    try {
      const results = this.accessor.source_read_dir(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read directory ${path} with source handle: ${err}`);
    }
  }
  /**
   * Read a file from an open source by `DirHandle`
   * @param source `SourceHandle` object
   * @param handle `DirHandle` object
   * @returns Array of `DirEntry` or `FileError`
   */
  sourceReadDirHandle(source, handle) {
    try {
      const results = this.accessor.source_read_dir_handle(source, handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read DirHandle ${JSON.stringify(handle.Directory)} with source handle: ${err}`);
    }
  }
  /**
   * Stat a file from an open source
   * @param source `SourceHandle` object
   * @param path Path to file to stat
   * @returns `EntryMeta` or `FileError`
   */
  sourceStat(source, path) {
    try {
      const results = this.accessor.source_stat(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat ${path} with source handle: ${err}`);
    }
  }
  /**
   * Stat a file from an open source by `FileHandle`
   * @param source `SourceHandle` object
   * @param handle `FileHandle` object
   * @returns `EntryMeta` or `FileError`
   */
  sourceStatHandle(source, handle) {
    try {
      const results = this.accessor.source_stat_handle(source, handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat FIleHandle ${JSON.stringify(handle.File)} with source handle: ${err}`);
    }
  }
  /**
   * Stat a file from an open source by `DirHandle`
   * @param source `SourceHandle` object
   * @param handle `DirHandle` object
   * @returns `EntryMeta` or `FileError`
   */
  sourceStatDirHandle(source, handle) {
    try {
      const results = this.accessor.source_stat_dir_handle(source, handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat DirHandle ${JSON.stringify(handle.Directory)} with source handle: ${err}`);
    }
  }
  /**
   * Glob files from an open source by path
   * @param source `SourceHandle` object
   * @param path Path pattern to glob
   * @returns Array of `GlobMatch` or `FileError`
   */
  sourceGlobfs(source, path) {
    try {
      const results = this.accessor.source_globfs(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not glob ${path} with source handle: ${err}`);
    }
  }
  /**
  * Open a file for reading. You can stream a file with this function
  * @param path Path to file to return a initialized `Reader` for
  * @param protocol `AccessorProtocol` to use use for the `Reader`
  * @returns `Reader` or `FileError`
  */
  sourceOpenReader(source, path) {
    try {
      const results = this.accessor.source_open_reader(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not open reader for path ${path} with source handle: ${err}`);
    }
  }
  /**
   * Open a `FileHandle` for reading. You can stream a file with this function
   * @param handle `FileHandle` to read
   * @returns `Reader` or `FileError`
   */
  sourceOpenReaderHandle(source, handle) {
    try {
      const results = this.accessor.source_open_reader_handle(source, handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not open reader for handle '${JSON.stringify(handle.File)}' with source handle: ${err}`);
    }
  }
  /**
   * Craft the full path to target file and select the correct accessor
   * @param path Path to target
   * @param protocol `AccessorProtocol` to use to open the target
   * @returns Path to target file with the correct accessor protocol
   */
  setTarget(path, protocol) {
    let target = path;
    switch (protocol) {
      case 1 /* Ntfs */: {
        target = `ntfs:${target}`;
        break;
      }
      case 2 /* Ext4 */: {
        target = `ext4:${target}`;
        break;
      }
      case 3 /* Zip */: {
        target = `zip:${target}`;
        break;
      }
      case 4 /* Raw */: {
        target = `raw:${target}`;
        break;
      }
      case 5 /* Qcow */: {
        target = `qcow:${target}`;
        break;
      }
      case 0 /* Host */: {
        break;
      }
      default:
        break;
    }
    return target;
  }
};

// ../../../../Downloads/main.ts
function main() {
  const accessor = new Accessor();
  let target = "C:\\Windows\\PFRO.log";
  if (platform().toLowerCase() !== "windows") {
    target = "/etc/passwd";
  }
  const results = accessor.globfs(target);
  if (results instanceof FileError) {
    console.log(JSON.stringify(results));
    return;
  }
  const max = 4;
  for (const entry of results) {
    if (entry.meta.kind !== "File" /* File */) {
      continue;
    }
    const value = readTextFile(entry.handle);
    if (value instanceof FileError) {
      return;
    }
    console.log(`JS Accessor Text Lines length: ${value.length}`);
    break;
  }
}
main();

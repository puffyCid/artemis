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

// ../../../artemis-api/src/filesystem/reader.ts
var Reader = class {
  /**
   * An `AccessorReader` that allows you to stream bytes
   * from a file.
   * 
   * Supports all `AccessorProtocol` values
   */
  reader;
  /**
   * Setup a `Reader` using the `Accessor`
   * @param reader `AccessorReader` return from `Accessor`
   */
  constructor(reader) {
    this.reader = reader;
  }
  /**
   * Function to read bytes from a file
   * @param offset Offset to start reading bytes
   * @param bytes How many bytes that should be read
   * @returns `Uint8Array` or `FileError`
   */
  readAt(offset, bytes) {
    if (offset < 0) {
      return new FileError(`READER`, `Cannot seek to negative offset ${offset}`);
    }
    if (bytes < 0) {
      return new FileError(`READER`, `Cannot read to negative bytes ${bytes}`);
    }
    try {
      const results = this.reader.read_at(offset, bytes);
      return results;
    } catch (err) {
      return new FileError(`READER`, `could not read bytes ${err} at offset ${offset}`);
    }
  }
  /**
   * Function to read bytes at current offset
   * @param bytes How many bytes to read
   * @returns `Uint8Array` or `FileError`
   */
  read(bytes) {
    if (bytes < 0) {
      return new FileError(`READER`, `Cannot read to negative bytes ${bytes}`);
    }
    try {
      const results = this.reader.read(bytes);
      return results;
    } catch (err) {
      return new FileError(`READER`, `could not read bytes ${err}`);
    }
  }
  /**
   * Seek to provided offset. Seek always start from start of the fiel
   * @param offset Where to jump to in the file
   * @returns Current offset or `FileError`
   */
  seek(offset) {
    if (offset < 0) {
      return new FileError(`READER`, `Cannot seek to negative offset ${offset}`);
    }
    try {
      const results = this.reader.seek(offset);
      return results;
    } catch (err) {
      return new FileError(`READER`, `could not seek ${err}`);
    }
  }
};

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
  sourceReadFileHandle(source, handle) {
    try {
      const results = this.accessor.source_read_file_handle(source, handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read FIleHandle ${JSON.stringify(handle.File)} with source handle: ${err}`);
    }
  }
  sourceReadDir(source, path) {
    try {
      const results = this.accessor.source_read_dir(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read directory ${path} with source handle: ${err}`);
    }
  }
  sourceReadDirHandle(source, handle) {
    try {
      const results = this.accessor.source_read_dir_handle(source, handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not read DirHandle ${JSON.stringify(handle.Directory)} with source handle: ${err}`);
    }
  }
  sourceStat(source, path) {
    try {
      const results = this.accessor.source_stat(source, path);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat ${path} with source handle: ${err}`);
    }
  }
  sourceStatHandle(source, handle) {
    try {
      const results = this.accessor.source_stat_handle(source, handle.File);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat FIleHandle ${JSON.stringify(handle.File)} with source handle: ${err}`);
    }
  }
  sourceStatDirHandle(source, handle) {
    try {
      const results = this.accessor.source_stat_dir_handle(source, handle.Directory);
      return results;
    } catch (err) {
      return new FileError(`ACCESSOR`, `could not stat DirHandle ${JSON.stringify(handle.Directory)} with source handle: ${err}`);
    }
  }
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
  setTarget(path, protocol) {
    let target = path;
    switch (protocol) {
      case 1 /* Ntfs */: {
        target = `ntfs:${target}`;
        break;
      }
      case 2 /* Zip */: {
        target = `zip:${target}`;
        break;
      }
      case 3 /* Raw */: {
        target = `raw:${target}`;
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
  const results = accessor.globfs("*");
  if (results instanceof FileError) {
    console.error(results);
    return;
  }
  for (const entry of results) {
    if (entry.meta.kind === "File" /* File */) {
      const accessor_reader = accessor.openReaderHandle(entry.handle);
      if (accessor_reader instanceof FileError) {
        console.error(accessor_reader);
        continue;
      }
      const reader = new Reader(accessor_reader);
      const bytes = reader.read(10);
      if (bytes instanceof FileError) {
        console.error(bytes);
        continue;
      }
      console.log(`Read ${bytes.length} from ${entry.meta.display_path}`);
      continue;
    }
    if (entry.meta.kind === "Directory" /* Directory */) {
      const files = accessor.readDirHandle(entry.handle);
      if (files instanceof FileError) {
        console.error(files);
        continue;
      }
      for (const file of files) {
        if (file.meta.kind === "File" /* File */) {
          const meta = accessor.statHandle(file.handle);
          if (meta instanceof FileError) {
            console.error(meta);
            continue;
          }
          console.log(`Meta for ${file.meta.display_path} is ${JSON.stringify(meta)}`);

          const test = accessor.stat(file.meta.display_path);
          if (test instanceof FileError) {
            console.error(test);
            continue;
          }
          console.log(`Meta for ${file.meta.display_path} is ${JSON.stringify(test)}`);
        }
      }
    }
  }
  const source = accessor.openSource("host:");
  if (source instanceof FileError) {
    console.error(source);
    return;
  }
  const matches = accessor.sourceGlobfs(source, "*");
  for (const entry of results) {
    if (entry.meta.kind === "File" /* File */) {
      const accessor_reader = accessor.sourceOpenReaderHandle(source, entry.handle);
      if (accessor_reader instanceof FileError) {
        console.error(accessor_reader);
        continue;
      }
      const reader = new Reader(accessor_reader);
      const bytes = reader.read(10);
      if (bytes instanceof FileError) {
        console.error(bytes);
        continue;
      }
      console.log(`Read ${bytes.length} from ${entry.meta.display_path}`);
      continue;
    }
    if (entry.meta.kind === "Directory" /* Directory */) {
      const files = accessor.sourceReadDirHandle(source, entry.handle);
      if (files instanceof FileError) {
        console.error(files);
        continue;
      }
      for (const file of files) {
        if (file.meta.kind === "File" /* File */) {
          const meta = accessor.sourceStatHandle(source, file.handle);
          if (meta instanceof FileError) {
            console.error(meta);
            continue;
          }
          console.log(`Source Meta for ${file.meta.display_path} is ${JSON.stringify(meta)}`);
        } else if (file.meta.kind === "Directory") {
          const meta = accessor.sourceStatDirHandle(source, file.handle);
          if (meta instanceof FileError) {
            console.error(meta);
            continue;
          }
          console.log(`Source Dir Meta for ${file.meta.display_path} is ${JSON.stringify(meta)}`);
        }
      }
    }
  }
}
main();

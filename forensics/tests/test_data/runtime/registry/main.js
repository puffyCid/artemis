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

// ../../../artemis-api/src/system/systeminfo.ts
function platform() {
  const data = js_platform();
  return data;
}

// ../../../artemis-api/src/environment/env.ts
function getEnvValue(key) {
  const data = js_env_value(key);
  return data;
}
function getSystemDrive() {
  if (!platform().toLowerCase().includes("windows")) {
    return "";
  }
  let drive = getEnvValue("SystemDrive");
  if (drive === "") {
    drive = "C:";
  }
  return drive;
}

// ../../../artemis-api/src/windows/errors.ts
var WindowsError = class extends ErrorBase {
};

// ../../../artemis-api/src/windows/registry.ts
function getRegistry(handle, regex = "", start_path = "") {
  try {
    const data = js_registry(handle.File, regex.toLowerCase(), start_path);
    return data;
  } catch (err) {
    return new WindowsError(
      "REGISTRY",
      `failed to parse registry file ${JSON.stringify(handle.File)}: ${err}`
    );
  }
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

// ../../../artemis-api/src/windows/registry/firewall_rules.ts
function firewallRules(protocol = 1 /* Ntfs */, alt_path) {
  const accessor = new Accessor();
  const drive = getSystemDrive();
  let path = `${drive}\\Windows\\System32\\config\\SYSTEM`;
  if (alt_path !== void 0) {
    path = alt_path;
  }
  const matches = accessor.globfs(path, protocol);
  if (matches instanceof FileError) {
    return new WindowsError("FIREWALL_RULES", `failed to glob for ${path}: ${matches}`);
  }
  const rules = [];
  for (const entry of matches) {
    if (entry.meta.kind !== "File" /* File */) {
      continue;
    }
    const reg_data = getRegistry(entry.handle);
    if (reg_data instanceof WindowsError) {
      return new WindowsError(`FIREWALL_RULES`, `failed to parse ${path}: ${reg_data}`);
    }
    for (const reg of reg_data) {
      if (!reg.path.endsWith("\\Services\\SharedAccess\\Parameters\\FirewallPolicy\\FirewallRules")) {
        continue;
      }
      for (const value of reg.values) {
        if (value.data_type !== "REG_SZ") {
          continue;
        }
        const entries = value.data.split("|");
        const rule = {
          action: "",
          active: false,
          direction: "Unknown" /* Unknown */,
          protocol: "Unknown" /* Unknown */,
          protocol_number: 0,
          local_port: 0,
          remote_port: 0,
          name: "",
          description: "",
          application: "",
          evidence: entry.meta.display_path,
          key_path: reg.path,
          last_modified: reg.last_modified,
          rule_version: "",
          profile: "",
          service: "",
          remote_address: [],
          registry_key_name: value.value,
          local_address: [],
          message: "",
          datetime: reg.last_modified,
          timestamp_desc: "Registry Last Modified",
          artifact: "Windows Firewall Rule",
          data_type: "windows:registry:firewallrule:entry"
        };
        for (const rule_values of entries) {
          if (rule_values.startsWith("v") && !rule_values.includes("=")) {
            rule.rule_version = rule_values;
            continue;
          }
          const key_value = rule_values.split("=");
          const key = key_value.at(0);
          if (key === "") {
            continue;
          }
          switch (key) {
            case "Action":
              rule.action = key_value.at(1) ?? "";
              break;
            case "Active":
              rule.active = JSON.parse(key_value.at(1)?.toLowerCase() ?? "false");
              break;
            case "Dir":
              rule.direction = getDirection(key_value.at(1) ?? "");
              break;
            case "Protocol":
              rule.protocol = getProtocol(key_value.at(1) ?? "");
              rule.protocol_number = Number(key_value.at(1));
              break;
            case "Profile":
              rule.profile = key_value.at(1) ?? "";
              break;
            case "RPort":
              rule.remote_port = Number(key_value.at(1));
              break;
            case "LPort":
              rule.local_port = Number(key_value.at(1));
              break;
            case "App":
              rule.application = key_value.at(1) ?? "";
              break;
            case "Svc":
              rule.service = key_value.at(1) ?? "";
              break;
            case "Name":
              rule.name = key_value.at(1) ?? "";
              rule.message = `Firewall Rule: ${rule.name}`;
              break;
            case "Desc":
              rule.description = key_value.at(1) ?? "";
              break;
            case "RA4":
              rule.remote_address.push(key_value.at(1) ?? "");
              break;
            case "RA6":
              rule.remote_address.push(key_value.at(1) ?? "");
              break;
            case "LA4":
              rule.local_address.push(key_value.at(1) ?? "");
              break;
            case "LA6":
              rule.local_address.push(key_value.at(1) ?? "");
              break;
            case void 0:
              break;
            default:
              rule[key] = key_value.at(1) ?? "";
              break;
          }
        }
        rules.push(rule);
      }
    }
  }
  return rules;
}
function getDirection(dir) {
  if (dir.toLowerCase() === "in") {
    return "Inbound" /* Inbound */;
  } else if (dir.toLowerCase() === "out") {
    return "Outbound" /* Outbound */;
  }
  return "Unknown" /* Unknown */;
}
function getProtocol(proto) {
  switch (proto) {
    case "6":
      return "TCP" /* TCP */;
    case "1":
      return "ICMP" /* ICMP */;
    case "17":
      return "UDP" /* UDP */;
    case "58":
      return "ICMP_v6" /* ICMP_v6 */;
    case "41":
      return "IPv6" /* IPV6 */;
    case "47":
      return "GRE" /* GRE */;
    case "2":
      return "IGMP" /* IGMP */;
    default:
      return "Unknown" /* Unknown */;
  }
}

// ../../../../Downloads/main.ts
function main() {
  const results = firewallRules();
  console.log(JSON.stringify(results));

  return results;
}
main();

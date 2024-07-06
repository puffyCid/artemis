import { console } from "ext:artemis/javascript/console.js";
import { filesystem } from "ext:artemis/javascript/filesystem.js";
import { environment } from "ext:artemis/javascript/environment.js";
import { encoding } from "ext:artemis/javascript/encoding.js";
import { system } from "ext:artemis/javascript/system.js";
import { time } from "ext:artemis/javascript/time.js";
import { requst } from "ext:artemis/javascript/http.js";
import { compression } from "ext:artemis/javascript/compression.js";
import { decryption } from "ext:artemis/javascript/decryption.js";

globalThis.console = console;
globalThis.fs = filesystem;
globalThis.env = environment;
globalThis.encoding = encoding;
globalThis.system = system;
globalThis.time = time;
globalThis.http = requst;
globalThis.compression = compression;
globalThis.decryption = decryption;

import { console } from "ext:artemis/javascript/console.js";
import { filesystem } from "ext:artemis/javascript/filesystem.js";
import { environment } from "ext:artemis/javascript/environment.js"
import { encoding } from "ext:artemis/javascript/encoding.js"

globalThis.console = console;
globalThis.fs = filesystem;
globalThis.env = environment;
globalThis.encoding = encoding;

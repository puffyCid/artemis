import { console } from "ext:artemis/javascript/console.js";
import { filesystem } from "ext:artemis/javascript/filesystem.js";
import { environment } from "ext:artemis/javascript/environment.js"

globalThis.console = console;
globalThis.fs = filesystem;
globalThis.env = environment;

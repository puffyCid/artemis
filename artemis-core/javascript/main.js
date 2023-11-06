import { console } from "ext:artemis/javascript/console.js";
import { filesystem } from "ext:artemis/javascript/filesystem.js";
import { environment } from "ext:artemis/javascript/environment.js";
import { encoding } from "ext:artemis/javascript/encoding.js";
import { systemInfo } from "ext:artemis/javascript/systeminfo.js";
import { time } from "ext:artemis/javascript/time.js";
import { requst } from "ext:artemis/javascript/http.js";

globalThis.console = console;
globalThis.fs = filesystem;
globalThis.env = environment;
globalThis.encoding = encoding;
globalThis.systemInfo = systemInfo;
globalThis.time = time;
globalThis.http = requst;
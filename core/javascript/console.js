const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
const { SafeArrayIterator } = primordials;
class Console {
    argsToMessage(...args) {
        return args.map((arg) => JSON.stringify(arg, (_, value) => typeof value === "bigint" ? value.toString() : value)).join(" ");
    }
    log = (...args) => {
        const result = this.argsToMessage(...args);
        core.ops.js_log("info", result);
        core.print(`[runtime]: ${result}\n`, false);
    };
    error = (...args) => {
        const result = this.argsToMessage(...args);
        core.ops.js_log("error", result);
        core.print(`[runtime-error]: ${this.argsToMessage(...args)}\n`, true);
    };
    info = (...args) => {
        const result = this.argsToMessage(...args);
        core.ops.js_log("info", result);
        core.print(`[runtime-info]: ${this.argsToMessage(...args)}\n`, true);
    };
    warn = (...args) => {
        const result = this.argsToMessage(...args);
        core.ops.js_log("warn", result);
        core.print(`[runtime-warn]: ${this.argsToMessage(...args)}\n`, true);
    };
    assert = (condition = false, ...args) => {
        if (condition) {
            return;
        }
        if (args.length === 0) {
            this.error("Assertion failed");
            return;
        }
        const [first, ...rest] = new SafeArrayIterator(args);
        if (typeof first === "string") {
            this.error(`Assertion failed: ${first}`, ...new SafeArrayIterator(rest));
            return;
        }
        this.error(`Assertion failed:`, ...new SafeArrayIterator(args));
    };
}
export const console = new Console();

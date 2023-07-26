const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
const { SafeArrayIterator } = primordials;
class Console {
    argsToMessage(...args) {
        return args.map((arg) => JSON.stringify(arg, (_, value) => typeof value === "bigint" ? value.toString() : value)).join(" ");
    }
    log = (...args) => {
        core.print(`[runtime]: ${this.argsToMessage(...args)}\n`, false);
    };
    error = (...args) => {
        core.print(`[runtime-error]: ${this.argsToMessage(...args)}\n`, true);
    };
    info = (...args) => {
        core.print(`[runtime-info]: ${this.argsToMessage(...args)}\n`, true);
    };
    warn = (...args) => {
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

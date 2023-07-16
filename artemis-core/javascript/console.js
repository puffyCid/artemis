//@ts-nocheck
const { core } = Deno;
class Console {
    argsToMessage(...args) {
        return args.map((arg) => JSON.stringify(arg)).join(" ");
    }
    log = (...args) => {
        core.print(`[runtime]: ${this.argsToMessage(...args)}\n`, false);
    };
    error = (...args) => {
        core.print(`[runtime-error]: ${this.argsToMessage(...args)}\n`, true);
    };
}
export const console = new Console();

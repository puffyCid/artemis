/**
 * Console Namespace defined at https://console.spec.whatwg.org/#console-namespace
 * Only part of the namespace is implemented
 *
 * Partially inspired by https://github.com/denoland/deno/blob/main/ext/console/01_console.js
 */

//@ts-ignore: Deno internals
const { core } = globalThis.Deno;
const primordials = globalThis.__bootstrap.primordials;
const { SafeArrayIterator } = primordials;

/**
 * @class Console used to print information to terminal
 */
class Console {
  private argsToMessage(...args: any[]) {
    return args.map((arg) =>
      JSON.stringify(
        arg,
        (_, value) => typeof value === "bigint" ? value.toString() : value,
      )
    ).join(" ");
  }

  log = (...args: any) => {
    const result = this.argsToMessage(...args);
    core.ops.js_log("info", result);
    core.print(`[runtime]: ${result}\n`, false);
  };

  error = (...args: any) => {
    const result = this.argsToMessage(...args);
    core.ops.js_log("error", result);
    core.print(`[runtime-error]: ${this.argsToMessage(...args)}\n`, true);
  };

  info = (...args: any) => {
    const result = this.argsToMessage(...args);
    core.ops.js_log("info", result);
    core.print(`[runtime-info]: ${this.argsToMessage(...args)}\n`, true);
  };

  warn = (...args: any) => {
    const result = this.argsToMessage(...args);
    core.ops.js_log("warn", result);
    core.print(`[runtime-warn]: ${this.argsToMessage(...args)}\n`, true);
  };

  assert = (condition = false, ...args: any) => {
    if (condition) {
      return;
    }

    if (args.length === 0) {
      this.error("Assertion failed");
      return;
    }

    const [ first, ...rest ] = new SafeArrayIterator(args);

    if (typeof first === "string") {
      this.error(
        `Assertion failed: ${first}`,
        ...new SafeArrayIterator(rest),
      );
      return;
    }

    this.error(`Assertion failed:`, ...new SafeArrayIterator(args));
  };
}

export const console = new Console();

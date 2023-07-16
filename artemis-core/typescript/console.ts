// Implement Console Namespace as defined at https://console.spec.whatwg.org/#console-namespace

//@ts-nocheck
const { core } = Deno;

class Console {
  private argsToMessage(...args: any[]) {
    return args.map((arg) => JSON.stringify(arg)).join(" ");
  }

  log = (...args: any) => {
    core.print(`[runtime]: ${this.argsToMessage(...args)}\n`, false);
  };

  error = (...args: any) => {
    core.print(`[runtime-error]: ${this.argsToMessage(...args)}\n`, true);
  };
  
}

export const console = new Console();


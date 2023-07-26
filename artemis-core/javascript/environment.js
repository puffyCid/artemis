const { core } = globalThis.Deno;
class Environment {
    environment = () => {
        return core.ops.js_env();
    };
    environmentValue = (key) => {
        return core.ops.js_env_value(key);
    };
}
export const environment = new Environment();

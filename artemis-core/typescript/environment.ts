//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

/**
 * @class Environment used to interact with the artemis process environment
 */
class Environment {
    /**
     * Collect all environment variables for the artemis process
     * @returns HashMap of strings for all environment variables 
     */
    environment = () => {
        return core.ops.js_env();
    };
    /**
     * Lookup a single Environment variable. Returns empty string if not found
     * @param key Environment variable to lookup
     * @returns Value of provided Environment variable
     */
    environmentValue = (key: string) => {
        return core.ops.js_env_value(key);
    };
}

export const environment = new Environment();

//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

/**
 * @class SystemInfo used to get metadata about the system
 */
class SystemInfo {
    /**
     * Get amount of time the system has been powered on in seconds
     * @returns uptime of system in seconds
     */
    uptime = () => {
        return core.ops.js_uptime();
    };
    /**
     * Return hostname of the system
     * @returns hostname of system
     */
    hostname = () => {
        return core.ops.js_hostname();
    };
    /**
     * Get the current OS version
     * @returns os version of system
     */
    osVersion = () => {
        return core.ops.js_os_version();
    };
    /**
     * Get the current kernel version 
     * @returns kernel version of system
     */
    kernelVersion = () => {
        return core.ops.js_kernel_version();
    };
    /**
     * Get the platform type of the system
     * @returns platform of the system. Ex: `Darwin` for macOS
     */
    platform = () => {
        return core.ops.js_kernel_version();
    };
    /**
     * Get disk information about the system
     * @returns array of disks from the system
     */
    disks = () => {
        return core.ops.js_disk_info();
    };
    /**
     * Get memory metadata about the system
     * @returns memory metadata from the system
     */
    memory = () => {
        return core.ops.js_memory_info();
    };
    /**
     * Get CPU information about the system
     * @returns array of CPU info from the system
     */
    cpu = () => {
        return core.ops.js_cpu_info();
    };
}

export const systemInfo = new SystemInfo();

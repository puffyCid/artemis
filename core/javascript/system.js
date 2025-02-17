const { core } = globalThis.Deno;
class System {
    uptime = () => {
        return core.ops.js_uptime();
    };
    hostname = () => {
        return core.ops.js_hostname();
    };
    osVersion = () => {
        return core.ops.js_os_version();
    };
    kernelVersion = () => {
        return core.ops.js_kernel_version();
    };
    platform = () => {
        return core.ops.js_platform();
    };
    disks = () => {
        return core.ops.js_disk_info();
    };
    memory = () => {
        return core.ops.js_memory_info();
    };
    cpu = () => {
        return core.ops.js_cpu_info();
    };
    execute = (command, args) => {
        return core.ops.js_command(command, args);
    };
}
export const system = new System();

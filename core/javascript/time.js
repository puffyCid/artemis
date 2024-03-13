const { core } = globalThis.Deno;
class Time {
    time_now = () => {
        return core.ops.js_time_now();
    };
    filetime_to_unixepoch = (filetime) => {
        return core.ops.js_filetime_to_unixepoch(filetime);
    };
    cocoatime_to_unixepoch = (cocoatime) => {
        return core.ops.js_cocoatime_to_unixepoch(cocoatime);
    };
    hfs_to_unixepoch = (hfstime) => {
        return core.ops.js_hfs_to_unixepoch(hfstime);
    };
    ole_automationtime_to_unixepoch = (oletime) => {
        return core.ops.js_ole_automationtime_to_unixepoch(oletime);
    };
    webkit_time_to_unixepoch = (webkittime) => {
        return core.ops.js_webkit_time_to_uniexepoch(webkittime);
    };
    fattime_utc_to_unixepoch = (fattime) => {
        return core.ops.js_fat_time_to_unixepoch(fattime);
    };
}
export const time = new Time();

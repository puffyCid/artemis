//@ts-ignore: Deno internals
const { core } = globalThis.Deno;

/**
 * @class Time used to convert timestamps to UnixEpoch
 */
class Time {
    /**
     * Function to return current time
     * @returns Current time in UnixEpoch seconds
     */
    time_now = () => {
        return core.ops.js_time_now();
    };
    /**
     * Convert Windows FILETIME to UnixEpoch seconds
     * @param filetime FILTIME timestamp
     * @returns UnixEpoch seconds
     */
    filetime_to_unixepoch = (filetime: BigInt) => {
        return core.ops.js_filetime_to_unixepoch(filetime);
    };
    /**
     * Convert macOS COCOA time to UnixEpoch seconds
     * @param cocoatime COCOA timestamp
     * @returns UnixEpoch seconds
     */
    cocoatime_to_unixepoch = (cocoatime: number) => {
        return core.ops.js_cocoatime_to_unixepoch(cocoatime);
    };
    /**
     * Convert macOS HFS+ time to UnixEpoch seconds
     * @param hfstime HFS+ timestamp
     * @returns UnixEpoch seconds
     */
    hfs_to_unixepoch = (hfstime: number) => {
        return core.ops.js_hfs_to_unixepoch(hfstime);
    };
    /**
     * Convert Windows OLE time to UnixEpoch seconds
     * @param oletime OLE timestamp
     * @returns UnixEpoch seconds
     */
    ole_automationtime_to_unixepoch = (oletime: number) => {
        return core.ops.js_ole_automationtime_to_unixepoch(oletime);
    };
    /**
     * Conver browser WebKit time to UnixEpoch
     * @param webkittime WebKit timestamp
     * @returns UnixEpoch seconds
     */
    webkit_time_to_unixepoch = (webkittime: number) => {
        return core.ops.js_webkit_time_to_uniexepoch(webkittime);
    };
    /**
     * Convert Windows FAT time byts to UnixEpoch
     * @param fattime FAT timestamp bytes
     * @returns UnixEpoch seconds
     */
    fattime_utc_to_unixepoch = (fattime: Uint8Array) => {
        return core.ops.js_fat_time_to_unixepoch(fattime);
    };
}

export const time = new Time();

import type { Settings } from "$lib/types/about";
import { Store } from "@tauri-apps/plugin-store";

/**
 * On page load, load the `settings.json` if available and get OpenSearch creds
 * @returns Promise `Settings` object
 */
export const load = async (): Promise<Settings> => {
    const store = await Store.load("settings.json", { autoSave: false });
    const settings: Settings = {
        user: await store.get("user") ?? "",
        creds: await store.get("creds") ?? "",
        domain: await store.get("domain") ?? "",
        index: await store.get("index") ?? "",
        indexes: [],
    };

    return settings;
};

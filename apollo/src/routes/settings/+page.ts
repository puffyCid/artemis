import { listIndexes } from "$lib/queries/indexes";
import type { Settings } from "$lib/types/about";
import { Store } from "@tauri-apps/plugin-store";

/**
 * On page load, load the `settings.json` if available and get OpenSearch creds
 * @returns Promise `Settings` object
 */
export const load = async (): Promise<Settings> => {
    const indexes = await listIndexes();
    const store = await Store.load("settings.json", { autoSave: false });
    const settings: Settings = {
        user: await store.get("user") ?? "",
        creds: await store.get("creds") ?? "",
        domain: await store.get("domain") ?? "",
        index: await store.get("index") ?? "",
        indexes: [],
    };

    if (indexes.status !== undefined) {
        return settings;
    }

    const ignore = [
        ".open",
        "collection_metadata",
        ".plugin",
        "security-auditlog-",
    ];

    for (const key in indexes as Record<string, unknown>) {
        let is_default = true;
        for (const entry of ignore) {
            if (key.startsWith(entry)) {
                is_default = true;
                break;
            }
            is_default = false;
        }

        if (!is_default) {
            settings.indexes.push(key);
        }
    }

    return settings;
};

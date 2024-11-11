import type { About } from "$lib/types/about";
import { invoke } from "@tauri-apps/api/core";

export const load = async (): Promise<About> => {
    return await invoke("about_me", {
        path:
            "/home/puffycid/Projects/artemis/app/src-tauri/tests/timelines/test.db",
    });
};

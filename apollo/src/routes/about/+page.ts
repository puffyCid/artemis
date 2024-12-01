import type { About } from "$lib/types/about";
import type { ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

export const load = async (): Promise<About | ErrorStatus> => {
    return await invoke("about_me", {
        path: "",
    });
};

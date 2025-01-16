import type { ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Function to timeline and upload data
 * @param index Name of index to upload
 * @param path Folder containing JSONL files
 * @returns Upload status
 */
export async function timelineFiles(
    index: string,
    path: string,
): Promise<unknown | ErrorStatus> {
    return await invoke("timeline_and_upload", { index, path });
}

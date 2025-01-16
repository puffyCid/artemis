import type { ErrorStatus, Update } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Function to apply a tag to a single row
 * @param index Name of index containing data
 * @param documentId Document ID that should be tagged
 * @param tagName Name of the tagged
 * @returns OpenSearch `Update` status or `ErrorStatus`
 */
export async function applyTag(
    index: string,
    documentId: string,
    tagName: string,
): Promise<Update | ErrorStatus> {
    return await invoke("apply_tag", { index, documentId, tagName });
}

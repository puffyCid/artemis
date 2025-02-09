import type { Artifacts, ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of indexes in OpenSearch
 * @returns Array of strings
 */
export async function listIndexes(): Promise<
    Record<string, unknown> | ErrorStatus
> {
    return await invoke("indexes");
}

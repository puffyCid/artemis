import type { Artifacts, ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of artifacts ingested in the OpenSearch index
 * @param index Index to query
 * @returns `Artifacts` object
 */
export async function listArtifacts(
    index: string,
): Promise<Artifacts | ErrorStatus> {
    return await invoke("list_artifacts", { index });
}

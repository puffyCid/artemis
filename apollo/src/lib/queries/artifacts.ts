import type { Artifacts, ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of artifacts ingested in the OpenSearch index
 * @returns `Artifacts` object
 */
export async function listArtifacts(): Promise<Artifacts | ErrorStatus> {
    return await invoke("list_artifacts");
}

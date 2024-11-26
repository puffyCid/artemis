import type { Metadata } from "$lib/types/indexes/metadata";
import type { ErrorStatus } from "$lib/types/search";
import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of artifacts ingested in the OpenSearch
 * @param path Path
 * @returns Array of aritfact strings
 */
export async function listArtifacts(
    path: string,
): Promise<Metadata | ErrorStatus> {
    return await invoke("metadata", {
        path: "",
    });
}

import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of artifacts ingested in the SQLITE database
 * @param path Path to the SQLITE database
 * @returns Array of aritfact strings
 */
export async function listArtifacts(path: string): Promise<string[]> {
    return await invoke("artifacts", {
        path: "./artemis/app/src-tauri/tests/timelines/test.db",
    });
}

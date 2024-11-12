import type { QueryState } from "$lib/types/queries";
import type { TimelineEntry } from "$lib/types/timeline";
import { invoke } from "@tauri-apps/api/core";

/**
 * Get list of timeline entries ingested in the SQLITE database
 * @param path Path to the SQLITE database
 * @param query Query to execute
 * @returns Array of TimelineEntry values
 */
export async function queryTimeline(
    path: string,
    query: QueryState,
): Promise<TimelineEntry[]> {
    return await invoke("query_timeline", {
        path:
            "/home/puffycid/Projects/artemis/app/src-tauri/tests/timelines/test.db",
        state: query,
    });
}

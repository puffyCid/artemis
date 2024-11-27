import { Ordering, type QueryState } from "$lib/types/queries";
import type { ErrorStatus } from "$lib/types/search";
import type { Hit, OpenSearchData, TimelineEntry } from "$lib/types/timeline";
import { invoke } from "@tauri-apps/api/core";
import type { State, TableHandler } from "@vincjo/datatables/server";
import { isError } from "./error";

/**
 * Get list of timeline entries ingested in OpenSearch
 * @param index Name of index
 * @param query Query to execute
 * @returns Array of TimelineEntry values
 */
export async function queryTimeline(
    index: string,
    query: QueryState,
): Promise<OpenSearchData | ErrorStatus> {
    return await invoke("query_timeline", {
        index: "",
        state: query,
    });
}

/**
 * Function to query the OpenSearch instance
 * @param state The DataTable state
 * @param index Name of OpenSearch index
 * @param rows_per_page Rows per page to display
 * @returns Array of `TimelineEntry` entries
 */
export async function queryCallback(
    state: State,
    index: string,
    table: TableHandler<TimelineEntry>,
): Promise<TimelineEntry[]> {
    const { currentPage, rowsPerPage, sort, filters } = state;
    const offset = (currentPage - 1) * rowsPerPage;

    state.rowsPerPage = table.rowsPerPage;

    let query_string = "match_all";
    if (filters != undefined) {
        query_string = "query_string";
    }
    let query: Record<string, any> = {
        "query": {
            [query_string]: {},
        },
    };
    for (const filter of filters || []) {
        query.query[query_string] = {
            "query": `${String(filter.field)}: ${filter.value}`,
        };
    }
    let ordering = Ordering.ASC;
    if (sort?.direction === "desc") {
        ordering = Ordering.DSC;
    }

    const query_limit = table.rowsPerPage;
    const results = await getTimeline(
        index,
        query_limit,
        offset,
        ordering,
        query,
    );

    if (isError(results)) {
        state.setTotalRows(0);
        return [];
    }

    state.setTotalRows(results.hits.total.value);
    const entries = [];
    for (const hit of results.hits.hits) {
        entries.push(hit._source);
    }

    return entries;
}

/**
 * List timeline entries
 * @param index OpenSearch index name
 * @param limit How many rows to return. Default is 100
 * @param offset Row to start at. Default is 0
 * @param order Ordering direction. Default is ascending
 * @param query Search query to execute
 */
async function getTimeline(
    index: string,
    limit = 100,
    offset = 0,
    order = Ordering.ASC,
    query: Record<string, unknown>,
): Promise<OpenSearchData | ErrorStatus> {
    const state: QueryState = {
        limit,
        offset,
        order,
        query,
    };
    console.log(query);

    return queryTimeline(index, state);
}

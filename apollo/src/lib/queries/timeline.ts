import { Ordering, type QueryState } from "$lib/types/queries";
import type { ErrorStatus } from "$lib/types/search";
import type { OpenSearchData, TimelineEntry } from "$lib/types/timeline";
import { invoke } from "@tauri-apps/api/core";
import type { State, TableHandler } from "@vincjo/datatables/server";
import { isError } from "./error";

/**
 * Get list of timeline entries ingested in OpenSearch
 * @param query Query to execute
 * @returns Array of TimelineEntry values
 */
export async function queryTimeline(
    query: QueryState,
): Promise<OpenSearchData | ErrorStatus> {
    return await invoke("query_timeline", {
        state: query,
    });
}

/**
 * Function to query the OpenSearch instance
 * @param state The DataTable state
 * @param rows_per_page Rows per page to display
 * @returns Array of `TimelineEntry` entries
 */
export async function queryCallback(
    state: State,
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
    let order_column = "datetime";
    if (sort != undefined) {
        if (sort.direction === "desc") {
            ordering = Ordering.DSC;
        }
        if (typeof sort.field === "string") {
            order_column = sort.field;
        }
    }

    const query_limit = table.rowsPerPage;
    const results = await getTimeline(
        query_limit,
        offset,
        order_column,
        ordering,
        query,
    );

    if (isError(results)) {
        state.setTotalRows(0);
        return [];
    }

    console.log(results.hits.total);
    state.setTotalRows(results.hits.total.value);
    const entries = [];
    for (const hit of results.hits.hits) {
        hit._source["_opensearch_document_id"] = hit._id;
        entries.push(hit._source);
    }

    return entries;
}

/**
 * List timeline entries
 * @param limit How many rows to return. Default is 100
 * @param offset Row to start at. Default is 0
 * @param order_column Column to sort by. Default is `datetime`
 * @param order Ordering direction. Default is ascending
 * @param query Search query to execute
 */
async function getTimeline(
    limit = 100,
    offset = 0,
    order_column = "datetime",
    order = Ordering.ASC,
    query: Record<string, unknown>,
): Promise<OpenSearchData | ErrorStatus> {
    const state: QueryState = {
        limit,
        offset,
        order_column,
        order,
        query,
    };

    return queryTimeline(state);
}

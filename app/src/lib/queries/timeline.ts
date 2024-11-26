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
 * @returns Array of `Hit` entries
 */
export async function queryCallback(
    state: State,
    index: string,
    table: TableHandler<Hit>,
    match: string,
): Promise<Hit[]> {
    const { currentPage, rowsPerPage, sort, filters } = state;
    const offset = (currentPage - 1) * rowsPerPage;

    state.rowsPerPage = table.rowsPerPage;

    let query: Record<string, any> = {
        "query": {
            [match]: {},
        },
    };
    for (const filter of filters || []) {
        query.query[match] = {
            [filter.field]: filter.value,
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
        return [];
    }

    state.setTotalRows(results.hits.total.value);

    return results.hits.hits;
}

/**
 * List timeline entries
 * @param path Path to SQLITE database
 * @param limit How many rows to return. Default is 10,000
 * @param offset Row to start at. Default is 0
 * @param column Column to filter on. Default is `message` with no filter
 * @param order_column Column to order on. Default is `datetime`
 * @param order Ordering direction. Default is ascending
 * @param filter Data to filter on. Default is no filter
 * @param comparison Comparison operator to use. Default is LIKE
 * @param json_key json key to filter for on raw json data. Default is empty string
 */
async function getTimeline(
    path: string,
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

    return queryTimeline(path, state);
}

import {
    ColumnName,
    Comparison,
    Ordering,
    type QueryState,
} from "$lib/types/queries";
import type { TimelineEntry, TimelineQuery } from "$lib/types/timeline";
import { invoke } from "@tauri-apps/api/core";
import type { State, TableHandler } from "@vincjo/datatables/server";

/**
 * Get list of timeline entries ingested in the SQLITE database
 * @param path Path to the SQLITE database
 * @param query Query to execute
 * @returns Array of TimelineEntry values
 */
export async function queryTimeline(
    path: string,
    query: QueryState,
): Promise<TimelineQuery> {
    return await invoke("query_timeline", {
        path:
            //"./artemis/app/src-tauri/tests/timelines/test.db",
            "/home/puffycid/Downloads/temp.db",

        state: query,
    });
}

/**
 * Function to query the SQLITE database based current DataTable state
 * @param state The DataTable state
 * @param db_path Path to the SQLITE database
 * @param rows_per_page Rows per page to display
 * @returns Array of `TimelineEntry` entries
 */
export async function queryCallback(
    state: State,
    db_path: string,
    table: TableHandler<TimelineEntry>,
): Promise<TimelineEntry[]> {
    const { currentPage, rowsPerPage, sort, filters } = state;
    const offset = (currentPage - 1) * rowsPerPage;

    let order_column = ColumnName.DATETIME;
    let ordering = Ordering.ASC;
    switch (sort?.field) {
        case "datetime":
            order_column = ColumnName.DATETIME;
        case "message":
            order_column = ColumnName.MESSAGE;
        case "timestamp_desc":
            order_column = ColumnName.TIMESTAMP_DESC;
        default:
            order_column = ColumnName.DATETIME;
    }

    if (sort?.direction === "desc") {
        ordering = Ordering.DSC;
    }
    state.rowsPerPage = table.rowsPerPage;

    const filter = (filters?.at(0)?.value as string) || "";
    let column_name = ColumnName.MESSAGE;
    switch (filters?.at(0)?.field) {
        case "datetime":
            column_name = ColumnName.DATETIME;
            break;
        case "message":
            column_name = ColumnName.MESSAGE;
            break;
        case "timestamp_desc":
            column_name = ColumnName.TIMESTAMP_DESC;
            break;
        case "data":
            column_name = ColumnName.DATA;
            break;
        default:
            column_name = ColumnName.MESSAGE;
    }

    const query_limit = table.rowsPerPage;
    const results = await getTimeline(
        db_path,
        query_limit,
        offset,
        column_name,
        order_column,
        ordering,
        filter,
    );

    const entries = results.data;
    state.setTotalRows(results.total_rows);
    if (filter === "") {
        state.setTotalRows(results.total_rows);
    } else {
        state.setTotalRows(results.total_rows);
        /**
         * If the current page is greater than all pages. Reset to first page
         * This will happen if we are on page 100 and we apply filter that returns less than 100 pages
         * We go back to page 1
         */
        if (state.currentPage > table.pages.length) {
            table.setPage(1);
        }
    }

    return entries;
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
    column = ColumnName.MESSAGE,
    order_column = ColumnName.DATETIME,
    order = Ordering.ASC,
    filter = "",
    comparison = Comparison.LIKE,
    json_key = "",
): Promise<TimelineQuery> {
    const query: QueryState = {
        limit,
        offset,
        filter,
        column,
        order,
        order_column,
        comparison,
        json_key,
    };
    console.log(query);

    return queryTimeline(path, query);
}

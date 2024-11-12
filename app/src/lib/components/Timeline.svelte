<script lang="ts">
    import { queryTimeline } from "$lib/queries/timeline";
    import {
        ColumnName,
        Comparison,
        Ordering,
        type QueryState,
    } from "$lib/types/queries";
    import type { TimelineEntry } from "$lib/types/timeline";
    import {
        TableHandler,
        Datatable,
        ThSort,
        ThFilter,
    } from "@vincjo/datatables";

    let entries: TimelineEntry[] = [];
    const table = new TableHandler(entries, {
        rowsPerPage: 100,
    });

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
        limit = 10000,
        offset = 0,
        column = ColumnName.MESSAGE,
        order_column = ColumnName.DATETIME,
        order = Ordering.ASC,
        filter = "",
        comparison = Comparison.LIKE,
        json_key = "",
    ) {
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

        entries = await queryTimeline(path, query);
        table.setRows(entries);
    }
</script>

<div class="col-span-full">
    {#await getTimeline("")}
        <p>Loading...</p>
    {:then}
        <Datatable basic {table}>
            <table>
                <thead>
                    <tr>
                        <ThSort {table} field="datetime">Datetime</ThSort>
                        <ThSort {table} field="timestamp_desc"
                            >Datetime Description</ThSort
                        >
                        <ThSort {table} field="message">Message</ThSort>
                    </tr>
                    <tr>
                        <ThFilter {table} field="datetime" />
                        <ThFilter {table} field="timestamp_desc" />
                        <ThFilter {table} field="message" />
                    </tr>
                </thead>
                <tbody>
                    {#each table.rows as row}
                        <tr>
                            <td>{row.datetime}</td>
                            <td>{row.timestamp_desc}</td>
                            <td>{row.message}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </Datatable>
    {:catch error}
        <li>Query failed</li>
    {/await}
</div>

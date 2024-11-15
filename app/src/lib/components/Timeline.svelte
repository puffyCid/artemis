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
        type State,
    } from "@vincjo/datatables/server";
    import Details from "./Details.svelte";

    let entries: TimelineEntry[] = [];
    const table = new TableHandler(entries, { rowsPerPage: 100 });

    table.createView([{ index: 3, name: "raw", isVisible: false }]);
    const search = table.createSearch();

    let query_limit = 100;
    let db_path = "";
    table.load((state: State) => queryTimelie(state));

    const queryTimelie = async (state: State): Promise<TimelineEntry[]> => {
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

        const results = await getTimeline(
            db_path,
            query_limit,
            offset,
            ColumnName.MESSAGE,
            order_column,
            ordering,
            filter,
        );
        if (search.value != "") {
            const filter_entries = [];
            for (const entry of results) {
                if (!JSON.stringify(entry["data"]).includes(search.value)) {
                    continue;
                }
                filter_entries.push(entry);
            }
            entries = filter_entries;
        } else {
            entries = results;
        }

        if (filter === "") {
            state.setTotalRows(391003);
        } else {
            state.setTotalRows(entries.length + entries.length + offset);
            if (state.currentPage > entries.length) {
                table.setPage(1);
            }
        }

        return entries;
    };
    table.invalidate();
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
    ): Promise<TimelineEntry[]> {
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

        //table.setRows(entries);
        return queryTimeline(path, query);
    }
</script>

<div class="col-span-full">
    <div class="w-full">
        <input
            type="text"
            bind:value={search.value}
            oninput={() => search.set()}
            placeholder="Search me"
        />
        <Datatable basic {table}>
            <table>
                <thead>
                    <tr>
                        <ThSort {table} field="datetime">Datetime</ThSort>
                        <ThSort {table} field="timestamp_desc"
                            >Datetime Description</ThSort
                        >
                        <ThSort {table} field="message">Message</ThSort>
                        <ThSort {table} field="raw">Raw Data</ThSort>
                    </tr>
                    <tr>
                        <ThFilter {table} field="datetime" />
                        <ThFilter {table} field="timestamp_desc" />
                        <ThFilter {table} field="message" />
                    </tr>
                </thead>
                <tbody>
                    {#each table.rows as row}
                        <Details data={row} />
                    {/each}
                </tbody>
            </table>
        </Datatable>
    </div>
</div>

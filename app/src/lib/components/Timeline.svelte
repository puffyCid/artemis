<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { TimelineEntry } from "$lib/types/timeline";
    import {
        TableHandler,
        Datatable,
        ThSort,
        ThFilter,
        type State,
    } from "@vincjo/datatables/server";
    import Details from "./Details.svelte";
    import Navigation from "./table/Navigation.svelte";
    import Search from "./table/Search.svelte";

    let entries: TimelineEntry[] = [];
    const table = new TableHandler(entries, { rowsPerPage: 100 });

    table.createView([{ index: 3, name: "raw", isVisible: false }]);

    let db_path = "";
    table.load((state: State) => queryCallback(state, db_path, table));

    let sort = table.createSort("message");
    table.invalidate();

    function sortColumn(name: string) {}
</script>

<div class="col-span-full">
    <div class="w-full">
        <Search {table} {db_path} />

        <Datatable {table}>
            <table>
                <thead>
                    <tr>
                        <ThSort {table} field="datetime">Datetime</ThSort>
                        <ThSort {table} field="timestamp_desc">
                            Datetime Description
                        </ThSort>
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
        <Navigation {table} {db_path} />
    </div>
</div>

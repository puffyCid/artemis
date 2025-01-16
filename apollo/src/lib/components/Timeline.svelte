<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { Hit, TimelineEntry } from "$lib/types/timeline";
    import {
        TableHandler,
        Datatable,
        ThSort,
        Th,
        ThFilter,
        type State,
    } from "@vincjo/datatables/server";
    import Details from "./Details.svelte";
    import Navigation from "./table/Navigation.svelte";
    import Search from "./table/Search.svelte";

    let entries: TimelineEntry[] = [];
    const table = new TableHandler(entries, { rowsPerPage: 100 });

    let index = "test";
    table.load((state: State) => queryCallback(state, index, table));

    table.invalidate();
</script>

<div class="col-span-full">
    <div class="w-full">
        <Search {table} {index} />

        <Datatable {table}>
            <table>
                <thead>
                    <tr>
                        <ThSort {table} field="datetime">Datetime</ThSort>
                        <ThSort {table} field="timestamp_desc">
                            Datetime Description
                        </ThSort>
                        <Th></Th>
                        <ThSort {table} field="message">Message</ThSort>
                    </tr>
                    <tr>
                        <ThFilter {table} field="datetime" />
                        <ThFilter {table} field="timestamp_desc" />
                        <Th></Th>
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
        <Navigation {table} {index} />
    </div>
</div>

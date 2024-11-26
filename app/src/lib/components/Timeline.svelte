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

    let entries: Hit[] = [];
    const table = new TableHandler(entries, { rowsPerPage: 100 });

    let index = "";
    table.load((state: State) =>
        queryCallback(state, index, table, "match_all"),
    );

    //let sort = table.createSort("_source");
    table.invalidate();

    function sortColumn(name: string) {}
</script>

<div class="col-span-full">
    <div class="w-full">
        <Search {table} {index} />

        <Datatable {table}>
            <table>
                <thead>
                    <tr>
                        <ThSort
                            {table}
                            field={(row) => {
                                console.log(row._source);
                                row._source.datetime;
                            }}>Datetime</ThSort
                        >
                        <ThSort
                            {table}
                            field={(row) => {
                                row._source.timestamp_desc;
                            }}
                        >
                            Datetime Description
                        </ThSort>
                        <ThSort
                            {table}
                            field={(row) => {
                                row._source.message;
                            }}>Message</ThSort
                        >
                    </tr>
                    <tr>
                        <ThFilter
                            {table}
                            field={(row) => {
                                row._source.datetime;
                            }}
                        />
                        <ThFilter
                            {table}
                            field={(row) => {
                                row._source.timestamp_desc;
                            }}
                        />
                        <ThFilter
                            {table}
                            field={(row) => {
                                row._source.message;
                            }}
                        />
                    </tr>
                </thead>
                <tbody>
                    {#each table.rows as row}
                        <Details data={row._source} />
                    {/each}
                </tbody>
            </table>
        </Datatable>
        <Navigation {table} {index} />
    </div>
</div>

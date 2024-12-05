<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { State, TableHandler } from "@vincjo/datatables/server";
    import Count from "./Count.svelte";
    import Navigation from "./Navigation.svelte";

    const props: { table: TableHandler; index: string } = $props();
    const table = props.table;
    const index = props.index;
    let value = $state();

    function rawSearch() {
        if (String(value).length === 0) {
            table.filters = [];
        } else {
            // Search all properties
            table.filters = [{ field: "*", value }];
        }
        table.load((state: State) => queryCallback(state, index, table));
        table.invalidate();
    }
</script>

<form onsubmit={() => rawSearch()}>
    <input
        class="input input-bordered w-full max-w-xs input-info"
        type="text"
        bind:value
        placeholder="Raw search"
    />
    <Count {table} {index} />
    <Navigation {table} {index} />
</form>

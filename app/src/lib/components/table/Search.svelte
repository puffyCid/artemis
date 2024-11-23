<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { State, TableHandler } from "@vincjo/datatables/server";
    import Count from "./Count.svelte";
    import Navigation from "./Navigation.svelte";

    const props: { table: TableHandler; db_path: string } = $props();
    const table = props.table;
    const db_path = props.db_path;
    let value = $state();

    function rawSearch() {
        console.log(`raw now`);
        if (String(value).length === 0) {
            table.filters = [];
        } else {
            table.filters = [{ field: "data", value }];
        }
        table.load((state: State) => queryCallback(state, db_path, table));
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
    <Count {table} {db_path} />
    <Navigation {table} {db_path} />
</form>

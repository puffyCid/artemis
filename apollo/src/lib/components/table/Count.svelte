<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { State, TableHandler } from "@vincjo/datatables/server";

    const props: { table: TableHandler } = $props();
    const table = props.table;

    const rows = [5, 10, 20, 50, 100];

    let count = $state(100);

    function setCount() {
        if (typeof count === "number") {
            table.rowsPerPage = count;
        } else {
            const limit = 100;
            table.rowsPerPage = limit;
        }
        table.load((state: State) => queryCallback(state, table));
        table.setPage(1);
        table.invalidate();
    }
</script>

<select
    class="select select-primary w-full max-w-xs p-2 m-3"
    bind:value={count}
    onchange={() => setCount()}
>
    {#each rows as option}
        <option value={option}>{option}</option>
    {/each}
</select>

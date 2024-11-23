<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { State, TableHandler } from "@vincjo/datatables/server";

    const props: { table: TableHandler; db_path: string } = $props();
    const table = props.table;
    const db_path = props.db_path;

    function jumpPage(page: number) {
        table.setPage(page);
        table.load((state: State) => queryCallback(state, db_path, table));
        //table.invalidate();
    }
    const { start, end, total } = $derived(table.rowCount);
</script>

<span class="m-3 p-2">
    <button
        class="btn btn-ghost"
        type="button"
        onclick={() => table.setPage("previous")}>Previous</button
    >
    {#each table.pagesWithEllipsis as page}
        <button
            class="btn btn-ghost"
            type="button"
            class:btn-active={page === table.currentPage}
            onclick={() => jumpPage(page)}>{page ?? "..."}</button
        >
    {/each}
    <button
        class="btn btn-ghost"
        type="button"
        onclick={() => table.setPage("next")}>Next</button
    >
    Showing {start} to {end} of {total} rows
</span>

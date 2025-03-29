<script lang="ts">
    import { queryCallback } from "$lib/queries/timeline";
    import type { State, TableHandler } from "@vincjo/datatables/server";
    import "cally";

    const props: { table: TableHandler } = $props();
    const table = props.table;

    let dates = "";
    let modalOpen = $state(false);
    /**
     * Toggle the modal for Calendar
     */
    function toggleModal() {
        modalOpen = !modalOpen;
    }

    /**
     * Grab the user selected date range
     */
    function onDate() {
        const entry = document.querySelector("calendar-range");
        if (
            entry != null &&
            entry.value.includes("/") &&
            dates != entry.value
        ) {
            dates = entry.value;
            table.load((state: State) => {
                if (state.filters === undefined) {
                    state.filters = [
                        { field: "timefilter", value: entry.value },
                    ];
                } else {
                    state.filters.push({
                        field: "timefilter",
                        value: entry.value,
                    });
                }
                return queryCallback(state, table);
            });
            table.invalidate();
            return toggleModal();
        }
    }
</script>

<span>
    <button
        class="btn btn-outline btn-secondary btn-wide rounded-sm"
        type="button"
        onclick={toggleModal}
    >
        Date filter
    </button>
</span>

<dialog class="modal" class:modal-open={modalOpen}>
    <div class="modal-box">
        <h3 class="text-lg font-bold">Select Date Range</h3>
        <form method="dialog" class="modal-action">
            <button
                class="btn btn-xs btn-circle btn-ghost absolute right-2 top-2"
                onclick={toggleModal}>X</button
            >
        </form>
        <calendar-range
            class="cally dropdown bg-base-100 rounded-box shadow-lg"
            type="button"
            onclick={onDate}
        >
            <calendar-month></calendar-month>
        </calendar-range>
    </div>
</dialog>

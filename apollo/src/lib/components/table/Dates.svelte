<script lang="ts">
    import "cally";
    
    let dateFilter = "";
    let modalOpen = $state(false);
    /**
     * Toggle the modal for Calendar
     */
    function toggleModal() {
        modalOpen = !modalOpen;

    }

    function onDate() {
        const test = document.querySelector("calendar-range");
        if(test != null && test.value.includes("/") && test.value != dateFilter) {
            dateFilter = test.value;
            console.log(`apply filter for: ${dateFilter}`);

            return toggleModal();
        }
        dateFilter = "";
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

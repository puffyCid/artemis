<script lang="ts">
    let { data }: { data: object } = $props();
    let modalOpen = $state(false);

    if (Array.isArray(data)) {
        data = data.sort();
    }

    /**
     * Toggle the modal for nested objects
     */
    function toggleModal() {
        modalOpen = !modalOpen;
    }
</script>

<td>
    <button class="btn btn-xs btn-outline" onclick={toggleModal}>
        Details
    </button>
    <dialog class="modal" class:modal-open={modalOpen}>
        <div class="modal-box" style="max-width:none;">
            <h3 class="text-lg font-bold">Additional Details</h3>
            <form method="dialog" class="modal-action">
                <button
                    class="btn btn-xs btn-circle btn-ghost absolute right-2 top-2"
                    onclick={toggleModal}>X</button
                >
            </form>
            <pre>
                {JSON.stringify(data, null, 2)}
            </pre>
        </div>
    </dialog>
</td>

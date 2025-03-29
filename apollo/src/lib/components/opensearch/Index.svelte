<script lang="ts">
    import { timelineFiles } from "$lib/queries/uploads";
    import { open } from "@tauri-apps/plugin-dialog";

    let modalOpen = $state(false);
    let index = $state();

    /**
     * Toggle the modal for closing folder selection
     */
    function toggleModal() {
        modalOpen = !modalOpen;
    }

    /**
     * Open provided folder and upload JSONL files
     */
    async function openFolder() {
        const file = await open({ directory: true });
        // We must have an index name
        if (file === null || typeof index != "string") {
            return;
        }
        const status = await timelineFiles(index, file);
        toggleModal();
    }
</script>

<div class="col-span-1 space-y-3 p-2">
    <button class="btn btn-outline btn-wide rounded-sm" onclick={toggleModal}>
        Upload Data
    </button>
    {#if modalOpen}
        <div>
            <dialog class="modal" class:modal-open={modalOpen}>
                <div class="modal-box space-y-3 p-3">
                    <h3 class="text-lg font-bold">Upload Data</h3>
                    <form method="dialog" class="modal-action">
                        <button
                            class="btn btn-xs btn-circle btn-ghost absolute right-2 top-2"
                            onclick={toggleModal}>X</button
                        >
                        <input
                            type="text"
                            placeholder="Provide OpenSearch Index name"
                            class="input input-bordered w-full"
                            bind:value={index}
                        />
                    </form>
                    <button
                        class="btn btn-outline btn-wide rounded-sm"
                        onclick={() => openFolder()}
                    >
                        Select Folder
                    </button>
                </div>
            </dialog>
        </div>
    {/if}
</div>

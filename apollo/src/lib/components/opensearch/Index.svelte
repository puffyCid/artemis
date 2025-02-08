<script lang="ts">
    import { timelineFiles } from "$lib/queries/uploads";
    import { open } from "@tauri-apps/plugin-dialog";

    let { modalOpen }: { modalOpen: boolean } = $props();
    let index = $state("apollo");

    /**
     * Toggle the modal for closing folder selection
     */
    function toggleModal() {
        modalOpen = !modalOpen;
    }

    async function openFolder() {
        const file = await open({ directory: true });
        if (file === null) {
            return;
        }
        console.log(`Lets open ${file}`);
        const status = await timelineFiles(index, file);
        toggleModal();
    }
</script>

<div>
    <dialog class="modal" class:modal-open={modalOpen}>
        <div class="modal-box">
            <h3 class="text-lg font-bold">Additional Details</h3>
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
                <button
                    class="btn btn-outline btn-wide rounded"
                    onclick={() => openFolder()}
                >
                    Select Folder
                </button>
            </form>
        </div>
    </dialog>
</div>

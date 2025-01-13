<script lang="ts">
    import type { TimelineEntry } from "$lib/types/timeline";

    export let data: TimelineEntry;

    let visible = false;
    let modalOpen = false;

    /**
     * Toggle timeline details
     */
    function viewData() {
        visible = !visible;
    }

    /**
     * Toggle the modal for nested objects
     */
    function toggleModal() {
        modalOpen = !modalOpen;
    }
</script>

<tr on:click={viewData}>
    <td>{data.datetime}</td>
    <td>{data.timestamp_desc}</td>
    <td>{data.message}</td>
</tr>

<!--Toggle to show all event details-->
{#if visible}
    <tr>
        <td colspan="3">
            <table class="table border m-2">
                <thead>
                    <tr>
                        <th>Key</th>
                        <th>Value</th>
                    </tr>
                </thead>
                <tbody>
                    {#each Object.entries(data) as [key, value]}
                        <tr>
                            <td>{key}</td>
                            {#if Array.isArray(value) && typeof value.at(0) != "object"}
                                <td>
                                    {#each value as entry}
                                        <div class="badge badge-outline">
                                            {entry}
                                        </div>
                                    {/each}
                                </td>
                                <!--If we have nested objects or array of objects. Use modal to display-->
                            {:else if typeof value === "object"}
                                <td>
                                    <button
                                        class="btn btn-xs btn-outline"
                                        on:click={toggleModal}
                                        >Details
                                    </button>
                                    <dialog
                                        class="modal"
                                        class:modal-open={modalOpen}
                                    >
                                        <div class="modal-box">
                                            <h3 class="text-lg font-bold">
                                                Additional Details
                                            </h3>
                                            <form
                                                method="dialog"
                                                class="modal-action"
                                            >
                                                <button
                                                    class="btn btn-xs btn-circle btn-ghost absolute right-2 top-2"
                                                    on:click={toggleModal}
                                                    >X</button
                                                >
                                            </form>
                                            <pre>
                                                {JSON.stringify(value, null, 2)}
                                            </pre>
                                        </div>
                                    </dialog>
                                </td>
                            {:else}
                                <td>{value}</td>
                            {/if}
                        </tr>
                    {/each}
                </tbody>
            </table>
        </td>
    </tr>
{/if}

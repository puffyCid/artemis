<script lang="ts">
    import type { TimelineEntry } from "$lib/types/timeline";
    import Tags from "./Tags.svelte";

    export let data: TimelineEntry;
    const tags = [
        { color: "red", name: "bad" },
        { color: "orange", name: "sus" },
        { color: "green", name: "legit" },
    ];

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

<tr>
    <td>{data.datetime}</td>
    <td>{data.timestamp_desc}</td>
    <td>
        <div class="dropdown dropdown-top">
            <div
                tabindex="0"
                role="button"
                class="btn btn-sm btn-square btn-ghost"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="1.5"
                    stroke="currentColor"
                    class="size-6"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M9.568 3H5.25A2.25 2.25 0 0 0 3 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 0 0 5.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 0 0 9.568 3Z"
                    />
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M6 6h.008v.008H6V6Z"
                    />
                </svg>
            </div>
            <ul
                tabindex="-1"
                class="dropdown-content p-2 bg-base-100 rounded-box z-[1] w-auto shadow"
            >
                <Tags {tags} document_id={data["_opensearch_document_id"] as string} />
            </ul>
        </div>
    </td>
    <td on:click={viewData}>
        <div class="join">
            {#if data["tags"] != undefined}
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    fill={tags.find((element) => element.name === data["tags"])
                        ?.color ?? "none"}
                    viewBox="0 0 24 24"
                    class="size-6 tooltip"
                    data-tip={data["tags"]}
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M9.568 3H5.25A2.25 2.25 0 0 0 3 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 0 0 5.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 0 0 9.568 3Z"
                    />
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M6 6h.008v.008H6V6Z"
                    />
                </svg>
            {/if}
            {data.message}
        </div>
    </td>
</tr>

<!--Toggle to show all event details-->
{#if visible}
    <tr>
        <td colspan="4">
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

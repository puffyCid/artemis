<script lang="ts">
    import type { TimelineEntry } from "$lib/types/timeline";

    export let data: TimelineEntry;

    let visible = false;
    /**
     * Toggle timeline details
     */
    function viewData() {
        visible = !visible;
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
                            <td>{value}</td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </td>
    </tr>
{/if}

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
    <td>{data["data"]}</td>
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
                        <!--If key is data, we skip it because it contains the raw JSON data-->
                        {#if key != "data"}
                            <tr>
                                <td>{key}</td>
                                <td>{value}</td>
                            </tr>
                        {/if}
                        <!--Now iterate through the JSON-->
                        {#if key === "data"}
                            {#each Object.entries(JSON.parse(value as string)) as [raw_key, raw_value]}
                            <!--Already got the artifact name above-->
                            {#if raw_key != "artifact"}
                                <tr>
                                    <td>{raw_key}</td>
                                    <td>{raw_value}</td>
                                </tr>
                            {/if}
                            {/each}
                        {/if}
                    {/each}
                </tbody>
            </table>
        </td>
    </tr>
{/if}

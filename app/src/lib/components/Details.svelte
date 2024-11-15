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
                        {:else}
                        <!--Now iterate through the JSON-->
                            {#each Object.entries(JSON.parse(value as string)) as [raw_key, raw_value]}
                            <!--Already got the keys above-->
                            {#if !["artifact", "message"].includes(raw_key) }
                                <tr>
                                    <td>{raw_key}</td>
                                    {#if raw_value instanceof Object}
                                        <td>{JSON.stringify(raw_value)}</td>
                                    {:else}
                                        <td>{raw_value}</td>
                                    {/if}
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

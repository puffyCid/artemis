<script lang="ts">
    import { listArtifacts } from "$lib/queries/artifacts";
    import { isError } from "$lib/queries/error";
    let artifacts: string[] = [];

    let index = "test";

    /**
     * List of artifacts
     */
    async function getList() {
        const status = await listArtifacts(index);
        if (isError(status)) {
            return;
        }

        // Loop and get each artifact name
        for (const entry of status.aggregations.artifacts.buckets) {
            artifacts.push(entry.key);
        }
    }
</script>

<div class="col-span-full">
    <div class="navbar bg-neutral text-neutral-content">
        <div class="navbar-start"></div>
        <div class="navbar-center hidden lg:flex">
            <ul class="menu menu-horizontal px-1">
                <li><a href="/timeline">Timeline</a></li>
                <li>
                    <details>
                        <summary>Artifacts</summary>
                        <ul class="bg-base-100 rounded-t-none p-2">
                            {#await getList()}
                                <li>Loading...</li>
                            {:then}
                                {#each artifacts as entry}
                                    <li class="text-black"><a>{entry}</a></li>
                                {/each}
                            {:catch error}
                                <li>Query failed: {error}</li>
                            {/await}
                        </ul>
                    </details>
                </li>
                <li><a>Settings</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </div>
        <div class="navbar-end"></div>
    </div>
</div>

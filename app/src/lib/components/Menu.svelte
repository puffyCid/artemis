<script lang="ts">
    import { listArtifacts } from "$lib/queries/artifacts";
    import { isError } from "$lib/queries/error";
    import type { ErrorStatus } from "$lib/types/search";
    let artifacts: string[];

    /**
     * List of artifacts
     * @param path path
     */
    async function getList(path: string) {
        const meta = await listArtifacts(path);
        if (isError(meta)) {
            return;
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
                            {#await getList("")}
                                <li>Loading...</li>
                            {:then}
                                {#each artifacts as entry}
                                    <li class="text-black"><a>{entry}</a></li>
                                {/each}
                            {:catch error}
                                <li>Query failed</li>
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

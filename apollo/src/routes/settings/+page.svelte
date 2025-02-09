<script lang="ts">
    import Menu from "$lib/components/Menu.svelte";
    import Index from "$lib/components/opensearch/Index.svelte";
    import type { Settings } from "$lib/types/about";
    import { load } from "@tauri-apps/plugin-store";

    let { data }: { data: Settings } = $props();

    let user = $state(data.user);
    let creds = $state(data.creds);
    let domain = $state(data.domain);
    let index = $state(data.index);

    /**
     * Save OpenSearch settings configuration
     */
    async function saveOpenSearch() {
        if (
            String(user).length === 0 &&
            String(creds).length === 0 &&
            String(domain).length === 0
        ) {
            return new Error("must provide username, creds, and domain");
        }
        const store = await load("settings.json", { autoSave: false });
        store.set("user", user);
        store.set("creds", creds);
        store.set("domain", domain);
        store.set("index", index);

        await store.save();
    }

    /**
     * Save the current OpenSearch Index to settings.json file
     */
    async function updateIndex() {
        const store = await load("settings.json", { autoSave: false });
        store.set("index", index);
        await store.save();
    }
</script>

<main class="grid grid-cols-2 space-x-2 space-y-5">
    <Menu />
    <div class="col-span-1 space-y-5 p-2">
        <input
            type="text"
            placeholder="OpenSearch IP or Domain"
            class="input input-bordered w-full input-primary"
            bind:value={domain}
        />
    </div>
    <div class="col-span-1 space-y-5 p-2">
        <label class="form-control w-full">
            <select
                class="select select-bordered select-primary"
                bind:value={index}
                onchange={() => updateIndex()}
            >
                <option disabled selected>Select index</option>
                {#each data.indexes as index}
                    <option>{index}</option>
                {/each}
            </select>
            <div class="label">
                <span class="label-text-alt">Current Index: {index}</span>
            </div>
        </label>
    </div>
    <div class="col-span-1 space-y-3 p-2">
        <label
            class="input input-bordered input-primary flex items-center gap-2"
        >
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 16 16"
                fill="currentColor"
                class="h-4 w-4 opacity-70"
            >
                <path
                    d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM12.735 14c.618 0 1.093-.561.872-1.139a6.002 6.002 0 0 0-11.215 0c-.22.578.254 1.139.872 1.139h9.47Z"
                />
            </svg>
            <input
                type="text"
                class="grow"
                placeholder="OpenSearch Username"
                bind:value={user}
            />
        </label>
        <label
            class="input input-bordered input-secondary flex items-center gap-2"
        >
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 16 16"
                fill="currentColor"
                class="h-4 w-4 opacity-70"
            >
                <path
                    fill-rule="evenodd"
                    d="M14 6a4 4 0 0 1-4.899 3.899l-1.955 1.955a.5.5 0 0 1-.353.146H5v1.5a.5.5 0 0 1-.5.5h-2a.5.5 0 0 1-.5-.5v-2.293a.5.5 0 0 1 .146-.353l3.955-3.955A4 4 0 1 1 14 6Zm-4-2a.75.75 0 0 0 0 1.5.5.5 0 0 1 .5.5.75.75 0 0 0 1.5 0 2 2 0 0 0-2-2Z"
                    clip-rule="evenodd"
                />
            </svg>
            <input
                type="password"
                class="grow"
                placeholder="OpenSearch Creds"
                bind:value={creds}
            />
        </label>
        <button
            class="btn btn-outline btn-primary btn-wide rounded"
            onclick={() => saveOpenSearch()}>Save Creds</button
        >
    </div>
    <Index />
</main>

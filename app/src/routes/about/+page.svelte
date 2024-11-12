<script lang="ts">
    import Menu from "$lib/components/Menu.svelte";
    import type { About } from "$lib/types/about";

    let { data }: { data: About } = $props();
    const kb = 1000;
    const mb = kb * 1000;
    const gb = mb * 1000;
</script>

<main class="grid grid-cols-4 space-x-2 space-y-5">
    <Menu />
    <div class="stats col-span-full shadow p-2">
        <div class="stat place-items-center">
            <div class="stat-value">{data.apollo}</div>
            <div class="stat-desc text-zinc-600">Apollo version</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{data.rust}</div>
            <div class="stat-desc text-zinc-600">Rust version</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{data.tauri}</div>
            <div class="stat-desc text-zinc-600">Tauri version</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{data.build}</div>
            <div class="stat-desc text-zinc-600">Build date</div>
        </div>
    </div>
    <div class="stats col-span-3 shadow p-2">
        <div class="stat place-items-center">
            <div class="stat-value">{data.artifacts}</div>
            <div class="stat-desc text-zinc-600">Artifacts Ingested</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{data.files}</div>
            <div class="stat-desc text-zinc-600">Files Read</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">
                {#if data.db < kb}
                    {Math.round(data.db)} bytes
                {:else if data.db < mb}
                    {Math.round(data.db / kb)} KBs
                {:else if data.db < gb}
                    {Math.round(data.db / mb)} MBs
                {:else}
                    {Math.round(data.db / gb)} GBs
                {/if}
            </div>
            <div class="stat-desc text-zinc-600">DB Size</div>
        </div>
    </div>
</main>

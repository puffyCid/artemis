<script lang="ts">
    import Menu from "$lib/components/Menu.svelte";
    import type { About } from "$lib/types/about";

    let { data }: { data: About } = $props();

    let memory_used = $state(0);
    let cpu = $state(0);
    let name = $state("");
    for (const key in data.resources.nodes) {
        const node = data.resources.nodes[key];
        memory_used = node.os.mem.used_percent;
        cpu = node.os.cpu.percent;
        name = node.name;
        break;
    }
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
            <div class="stat-value">{memory_used}%</div>
            <div class="stat-desc text-zinc-600">OS Memory Used</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{cpu}%</div>
            <div class="stat-desc text-zinc-600">OS CPU Usage</div>
        </div>
        <div class="stat place-items-center">
            <div class="stat-value">{name}</div>
            <div class="stat-desc text-zinc-600">Cluster Name</div>
        </div>
    </div>
</main>

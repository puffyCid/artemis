<script lang="ts">
  import { timelineFiles } from "$lib/queries/uploads";
  import { open } from "@tauri-apps/plugin-dialog";

  async function openFolder() {
    const file = await open({ directory: true });
    if (file === null) {
      return;
    }
    console.log(`Lets open ${file}`);
    const status = timelineFiles("test", file);
  }
</script>

<main class="grid grid-cols-1 h-screen place-items-center">
  <ol class="list-decimal">
    <li>Click Open Folder</li>
    <li>Select the folder containing the JSONL output from Artemis</li>
  </ol>
  <div class="flex col-span-1 items-center">
    <button
      class="btn btn-outline btn-wide rounded"
      onclick={() => openFolder()}
    >
      Open Folder
    </button>
  </div>
  <div class="flex col-span-1 items-center">
    <button class="btn btn-outline btn-wide rounded">
      Connect to OpenSearch
    </button>
  </div>
  <p><a href="/about">Test</a></p>
</main>

<script lang="ts">
  import Index from "$lib/components/opensearch/Index.svelte";
</script>

<main class="grid grid-cols-1 h-screen place-items-center">
  <ol class="list-decimal">
    <li>Click Open Folder</li>
    <li>Provide an OpenSearch Index name</li>
    <li>Select the folder containing the JSONL output from Artemis</li>
  </ol>
  <Index />
  <div class="flex col-span-1 items-center">
    <button class="btn btn-outline btn-wide rounded-sm">
      Connect to OpenSearch
    </button>
  </div>
  <p><a href="/about">Test</a></p>
</main>

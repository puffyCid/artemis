<script lang="ts">
    import { applyTag } from "$lib/queries/tags";

    const props: {
        tags: { color: string; name: string }[];
        document_id: string;
    } = $props();
    const document_id = props.document_id;
    const tags = props.tags;

    /**
     * Function to tag a row
     * @param tag The tag name to apply
     */
    async function tagRow(tag: string) {
        const status = await applyTag("test", document_id, tag);
    }
</script>

<li>
    <div class="join">
        {#each tags as tag}
            <button
                class="btn btn-sm btn-ghost tooltip"
                aria-label={tag.name}
                data-tip={tag.name}
                onclick={() => tagRow(tag.name)}
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill={tag.color}
                    class="size-6"
                >
                    <path
                        fill-rule="evenodd"
                        d="M5.25 2.25a3 3 0 0 0-3 3v4.318a3 3 0 0 0 .879 2.121l9.58 9.581c.92.92 2.39 1.186 3.548.428a18.849 18.849 0 0 0 5.441-5.44c.758-1.16.492-2.629-.428-3.548l-9.58-9.581a3 3 0 0 0-2.122-.879H5.25ZM6.375 7.5a1.125 1.125 0 1 0 0-2.25 1.125 1.125 0 0 0 0 2.25Z"
                        clip-rule="evenodd"
                    />
                </svg>
            </button>
        {/each}
    </div>
</li>

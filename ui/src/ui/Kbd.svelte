<script lang="ts">
  let { keys }: { keys: string | string[] } = $props();
  const list = $derived(Array.isArray(keys) ? keys : [keys]);
</script>

{#each list as k, i (i)}<kbd class="kbd">{k}</kbd>{/each}

<style>
  .kbd {
    display: inline-grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    padding: 0 var(--s-2);
    border: 1px solid var(--control);
    border-bottom-width: 2px;
    border-radius: var(--r-sm);
    background: var(--sheet);
    color: var(--muted);
    font: 600 var(--fs-xs) / 1 var(--font);
  }
  /* Inside a filled button or a tooltip the key takes the surrounding ink. */
  :global([data-kbd-inherit]) .kbd {
    background: transparent;
    border-color: currentColor;
    color: inherit;
    opacity: 0.85;
  }
</style>

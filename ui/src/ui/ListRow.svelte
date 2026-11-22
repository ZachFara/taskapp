<script lang="ts">
  import type { Snippet } from 'svelte';

  // A focusable, selectable row for short lists (cycles, pinned). Focus and selection are owned by the caller.
  let { focused = false, selected = false, onpick, children }: { focused?: boolean; selected?: boolean; onpick?: () => void; children?: Snippet } = $props();
</script>

<button type="button" class="lrow" data-focused={focused} data-selected={selected} aria-pressed={selected} onclick={onpick}>
  {@render children?.()}
</button>

<style>
  .lrow {
    display: flex;
    align-items: center;
    gap: var(--s-5);
    width: 100%;
    height: var(--row-h);
    padding: 0 var(--s-7);
    border: 0;
    background: transparent;
    color: var(--ink);
    text-align: left;
    transition: background var(--dur-1) var(--ease);
  }
  .lrow:hover { background: var(--active); }
  .lrow[data-selected='true'] { background: var(--selection); }
  .lrow[data-focused='true'] { box-shadow: inset 2px 0 0 var(--signal); }
</style>

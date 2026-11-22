<script lang="ts">
  import type { Snippet } from 'svelte';

  // The one header every settings section starts with. The section name lives in the page header.
  let { lede, actions }: { lede: string; actions?: Snippet } = $props();
</script>

<header class="sec-head">
  <p>{lede}</p>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .sec-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-5) var(--s-6);
    padding-bottom: var(--s-6);
    border-bottom: 1px solid var(--border);
  }
  .sec-head p { flex: 1; min-width: 16rem; max-width: 56ch; color: var(--muted); line-height: var(--lh-body); }
  .actions { display: flex; align-items: center; gap: var(--s-4); }
</style>

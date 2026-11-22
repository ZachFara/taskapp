<script lang="ts">
  import type { Snippet } from 'svelte';

  // A small label chip. `color` is a palette key and tints the chip; without it the chip is neutral.
  let { color, children }: { color?: string; children: Snippet } = $props();
</script>

<span class="pill" style:--tint={color ? `var(--c-${color})` : undefined} data-tinted={color ? '' : undefined}>
  {#if color}<span class="dot"></span>{/if}{@render children()}
</span>

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--s-3);
    height: 20px;
    padding: 0 var(--s-3);
    border-radius: var(--r-sm);
    background: var(--active);
    font-size: var(--fs-sm);
    font-weight: 500;
    white-space: nowrap;
  }
  .pill[data-tinted] { background: color-mix(in srgb, var(--tint) 16%, var(--sheet)); }
  .dot { width: 6px; height: 6px; border-radius: var(--r-round); background: var(--tint); }
</style>

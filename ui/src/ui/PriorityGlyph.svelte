<script lang="ts">
  // Contour rings: 0 none (dashed), 1 low, 2 medium, 3 high, 4 urgent (all rings, a core, signal colour).
  let { level, size = 16, label }: { level: number; size?: number; label?: string } = $props();
  const names = ['No priority', 'Low', 'Medium', 'High', 'Urgent'];
  const n = $derived(Math.max(0, Math.min(4, Math.round(level))));
  const rings = [[7, 6.02], [4.6, 3.96], [2.4, 2.06]] as const;
  const shown = $derived(n === 0 ? 0 : Math.min(n, 3));
</script>

<svg
  class="glyph"
  class:urgent={n === 4}
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.3"
  role="img"
  aria-label={label ?? names[n]}
  data-level={n}
>
  {#if n === 0}
    <ellipse cx="8" cy="8" rx="5.6" ry="4.8" transform="rotate(-24 8 8)" stroke-dasharray="1.6 2" opacity="0.7" />
  {/if}
  {#each rings.slice(0, shown).reverse() as [rx, ry], i (i)}
    <ellipse cx="8" cy="8" {rx} {ry} transform="rotate(-24 8 8)" />
  {/each}
  {#if n === 4}
    <circle cx="8" cy="8" r="1.4" fill="currentColor" stroke="none" />
  {/if}
</svg>

<style>
  .glyph { flex: none; display: block; color: var(--ink); }
  .urgent { color: var(--signal); }
</style>

<script lang="ts">
  // Three bars of rising height: 0 none (all faint), 1 low, 2 medium, 3 high (that many solid), 4 urgent (all three, signal colour).
  let { level, size = 16, label }: { level: number; size?: number; label?: string } = $props();
  const names = ['No priority', 'Low', 'Medium', 'High', 'Urgent'];
  const n = $derived(Math.max(0, Math.min(4, Math.round(level))));
  const shown = $derived(Math.min(n, 3));
  const bars = [
    { x: 2, y: 10, h: 4 },
    { x: 6, y: 7, h: 7 },
    { x: 10, y: 4, h: 10 },
  ];
</script>

<svg
  class="glyph"
  class:urgent={n === 4}
  width={size}
  height={size}
  viewBox="0 0 16 16"
  role="img"
  aria-label={label ?? names[n]}
  data-level={n}
>
  {#each bars as b, i (i)}
    <rect x={b.x} y={b.y} width="3" height={b.h} rx=".6" class:on={i < shown} />
  {/each}
</svg>

<style>
  .glyph { flex: none; display: block; color: var(--ink); }
  .urgent { color: var(--signal); }
  rect { fill: currentColor; opacity: 0.25; }
  rect.on { opacity: 1; }
</style>

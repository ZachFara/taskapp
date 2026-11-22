<script lang="ts">
  import type { StatusCategory } from '../lib/bindings';

  // A cairn seen from the side. The silhouette is always the same three stones; how many are solid says how far
  // along the work is (one, two, three of three; the rest are faint), so the states stay apart at 16px.
  // Backlog is a dashed ghost of the stack; canceled is the empty stack struck through.
  let { category, size = 16, label }: { category: StatusCategory; size?: number; label?: string } = $props();
  const stones = [
    { x: 1.2, y: 10.1, w: 13.6, h: 4.9 },
    { x: 3.1, y: 5.3, w: 9.8, h: 4.2 },
    { x: 5.2, y: 1.3, w: 5.8, h: 3.4 },
  ];
  const solid = $derived({ Backlog: 0, Unstarted: 1, Started: 2, Completed: 3, Canceled: 0 }[category]);
  const ghost = $derived(category === 'Backlog');
</script>

<svg class="glyph" width={size} height={size} viewBox="0 0 16 16" role="img" aria-label={label ?? category} data-category={category}>
  {#each stones as s, i (i)}
    {#if i < solid}
      <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={s.h / 2} fill="currentColor" />
    {:else}
      <rect
        x={s.x + (ghost ? 0.55 : 0)}
        y={s.y + (ghost ? 0.55 : 0)}
        width={s.w - (ghost ? 1.1 : 0)}
        height={s.h - (ghost ? 1.1 : 0)}
        rx={s.h / 2}
        fill={ghost ? 'none' : 'currentColor'}
        fill-opacity="0.2"
        stroke={ghost ? 'currentColor' : 'none'}
        stroke-width="1.1"
        stroke-dasharray="2 1.6"
      />
    {/if}
  {/each}
  {#if category === 'Canceled'}
    <path d="M2 2.2l12 11.6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
  {/if}
</svg>

<style>
  .glyph { flex: none; display: block; color: var(--stone); }
</style>

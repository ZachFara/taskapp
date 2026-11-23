<script lang="ts">
  import { line, scales } from './geometry';

  // A word-sized trend line: no axes, one end dot. Fewer than two points draws nothing.
  let { values, label, w = 72, h = 20 }: { values: number[]; label: string; w?: number; h?: number } = $props();
  const pts = $derived.by(() => {
    const top = Math.max(1, ...values);
    const s = scales(values.length, top, { w, h, left: 3, right: 3, top: 3, bottom: 3 });
    return values.map((v, i) => [s.x(i), s.y(v)] as [number, number]);
  });
</script>

{#if values.length >= 2}
  <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} role="img" aria-label={label}>
    <path class="spark" d={line(pts)} />
    <circle class="end" cx={pts.at(-1)![0]} cy={pts.at(-1)![1]} r="2.5" />
  </svg>
{/if}

<style>
  svg { display: block; flex: none; }
  .spark { fill: none; stroke: var(--accent); stroke-width: 1.5; stroke-linejoin: round; }
  .end { fill: var(--accent); }
</style>

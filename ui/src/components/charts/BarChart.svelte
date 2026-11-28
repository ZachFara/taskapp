<script lang="ts">
  import Button from '../../ui/Button.svelte';
  import ChartTable from './ChartTable.svelte';
  import EmptyChart from './EmptyChart.svelte';
  import { fit } from './fit';
  import { niceTicks, scales } from './geometry';

  // Completed per period (cycle or week): square columns on a zero baseline, the latest period in full colour,
  // the earlier ones quieter, the average as a dashed reference line.
  let {
    bars,
    unit,
    title,
    format = (n: number) => String(Math.round(n * 10) / 10),
  }: { bars: { label: string; value: number }[]; unit: string; title: string; format?: (n: number) => string } = $props();

  let width = $state(0);
  let table = $state(false);
  const w = $derived(width || 480);
  const h = 150;
  const box = $derived({ w, h, left: 32, right: 8, top: 14, bottom: 22 });
  const ticks = $derived(niceTicks(Math.max(0, ...bars.map((b) => b.value)), 3));
  const top = $derived(ticks.at(-1)!);
  const base = $derived(h - box.bottom);
  const slot = $derived((w - box.left - box.right) / Math.max(1, bars.length));
  const bw = $derived(Math.min(40, slot * 0.6));
  // A label needs about 36 px; with narrower bars only every nth period gets one, so dates do not run together.
  const every = $derived(Math.max(1, Math.ceil(36 / slot)));
  const s = $derived(scales(2, top, box));
  const avg = $derived(bars.length > 1 ? bars.reduce((a, b) => a + b.value, 0) / bars.length : null);
  const text = $derived(`${title}: ${bars.map((b) => `${b.label} ${format(b.value)}`).join(', ')} ${unit}.`);
</script>

{#if bars.length === 0}
  <EmptyChart />
{:else}
  <figure use:fit={(w) => (width = w)}>
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} role="img" aria-label={text}>
      {#each ticks as t, i (t)}
        <line class={i === 0 ? 'base' : 'grid'} x1={box.left} x2={w - box.right} y1={s.y(t)} y2={s.y(t)} />
        <text class="tick" x={box.left - 6} y={s.y(t) + 4} text-anchor="end">{format(t)}</text>
      {/each}
      {#each bars as b, i (b.label)}
        {@const x = box.left + slot * i + (slot - bw) / 2}
        <rect class="bar" class:latest={i === bars.length - 1} {x} y={s.y(b.value)} width={bw} height={base - s.y(b.value)} />
        <!-- Dense charts label only non-zero bars, so empty days do not read as a row of 0s. -->
        {#if b.value !== 0 || slot >= 36}<text class="val" x={x + bw / 2} y={s.y(b.value) - 4} text-anchor="middle">{format(b.value)}</text>{/if}
        {#if i % every === 0}<text class="tick" x={x + bw / 2} y={h - 5} text-anchor="middle">{b.label}</text>{/if}
      {/each}
      {#if avg !== null}
        <line class="avg" x1={box.left} x2={w - box.right} y1={s.y(avg)} y2={s.y(avg)} />
      {/if}
    </svg>
    <div class="more"><Button aria-pressed={table} onclick={() => (table = !table)}>{table ? 'Hide table' : 'View as table'}</Button></div>
    {#if table}<ChartTable caption={title} head={['Period', 'Completed']} rows={bars.map((b) => [b.label, format(b.value)])} />{/if}
  </figure>
{/if}

<style>
  figure { margin: 0; display: grid; gap: var(--s-4); min-width: 0; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--border); stroke-width: 1; }
  .base { stroke: var(--control); stroke-width: 1.5; }
  .tick { fill: var(--muted); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .val { fill: var(--ink); font-size: var(--fs-xs); font-weight: 600; font-variant-numeric: tabular-nums; }
  .bar { fill: var(--accent); fill-opacity: 0.45; }
  .bar.latest { fill-opacity: 1; }
  .avg { stroke: var(--muted); stroke-opacity: 0.6; stroke-width: 1; stroke-dasharray: 4 4; }
  .more { display: flex; }
</style>

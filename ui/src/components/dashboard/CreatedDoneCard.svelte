<script lang="ts">
  import Button from '../../ui/Button.svelte';
  import ChartTable from '../charts/ChartTable.svelte';
  import EmptyChart from '../charts/EmptyChart.svelte';
  import { fit } from '../charts/fit';
  import { niceTicks, scales } from '../charts/geometry';
  import type { CardProps } from './cards';

  // Created and completed per bucket. Always counts: the engine's created is a count, so both bars are issues.
  let { metrics: m }: CardProps = $props();
  let width = $state(0);
  let table = $state(false);
  const rows = $derived(m.buckets.map((b) => ({ label: b.start.slice(5), created: b.created, completed: b.completed })));
  const w = $derived(width || 480);
  const h = 150;
  const box = $derived({ w, h, left: 32, right: 8, top: 14, bottom: 22 });
  const ticks = $derived(niceTicks(Math.max(0, ...rows.flatMap((r) => [r.created, r.completed])), 3));
  const s = $derived(scales(2, ticks.at(-1)!, box));
  const base = $derived(h - box.bottom);
  const slot = $derived((w - box.left - box.right) / Math.max(1, rows.length));
  const bw = $derived(Math.min(18, slot * 0.35));
  const every = $derived(Math.max(1, Math.ceil(36 / slot)));
  const text = $derived(`Created and completed per ${m.bucket.toLowerCase()}, issues: ${rows.map((r) => `${r.label} ${r.created} created, ${r.completed} completed`).join('; ')}.`);
</script>

{#if rows.every((r) => r.created === 0 && r.completed === 0)}
  <EmptyChart />
{:else}
  <figure use:fit={(x) => (width = x)}>
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} role="img" aria-label={text}>
      {#each ticks as t, i (t)}
        <line class={i === 0 ? 'base' : 'grid'} x1={box.left} x2={w - box.right} y1={s.y(t)} y2={s.y(t)} />
        <text class="tick" x={box.left - 6} y={s.y(t) + 4} text-anchor="end">{t}</text>
      {/each}
      {#each rows as r, i (r.label)}
        {@const x = box.left + slot * i + slot / 2}
        <rect class="created" x={x - bw} y={s.y(r.created)} width={bw} height={base - s.y(r.created)} />
        <rect class="completed" {x} y={s.y(r.completed)} width={bw} height={base - s.y(r.completed)} />
        {#if i % every === 0}<text class="tick" {x} y={h - 5} text-anchor="middle">{r.label}</text>{/if}
      {/each}
    </svg>
    <p class="key"><span class="sw created"></span>Created <span class="sw completed"></span>Completed</p>
    <div class="more"><Button aria-pressed={table} onclick={() => (table = !table)}>{table ? 'Hide table' : 'View as table'}</Button></div>
    {#if table}<ChartTable caption="Created and completed, issues" head={['Period', 'Created', 'Completed']} rows={rows.map((r) => [r.label, r.created, r.completed])} />{/if}
  </figure>
{/if}

<style>
  figure { margin: 0; display: grid; gap: var(--s-4); min-width: 0; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--border); stroke-width: 1; }
  .base { stroke: var(--control); stroke-width: 1.5; }
  .tick { fill: var(--muted); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .created { fill: var(--muted); fill-opacity: 0.5; }
  .completed { fill: var(--accent); }
  .key { margin: 0; color: var(--muted); font-size: var(--fs-sm); display: flex; align-items: center; gap: var(--s-2); }
  .sw { display: inline-block; width: var(--s-3); height: var(--s-3); border-radius: var(--r-sm); }
  .sw.created { background: var(--muted); opacity: 0.5; }
  .sw.completed { background: var(--accent); }
  .more { display: flex; }
</style>

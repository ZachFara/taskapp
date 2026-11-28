<script lang="ts">
  import { selectedIssueId } from '../../lib/stores.svelte';
  import { fit } from '../charts/fit';
  import { niceTicks, scales } from '../charts/geometry';
  import type { CardProps } from './cards';

  // One dot per finished issue: the day it finished against how many days it took. Click or Enter opens the issue.
  let { metrics: m }: CardProps = $props();
  const plural = (n: number, unit: string) => `${n} ${unit}${n === 1 ? '' : 's'}`;

  let width = $state(0);
  const w = $derived(width || 360);
  const h = 160;
  const box = { left: 28, right: 8, top: 12, bottom: 22 };
  const pts = $derived(m.cycle_time.points);
  const ticks = $derived(niceTicks(Math.max(0, ...pts.map((p) => p.days)), 3));
  const top = $derived(ticks.at(-1)!);
  const span = $derived(Math.max(1, (Date.parse(`${m.to}T00:00:00Z`) - Date.parse(`${m.from}T00:00:00Z`)) / 86_400_000));
  const s = $derived(scales(span + 1, top, { ...box, w, h }));
  const dayX = (date: string) => s.x(Math.min(span, Math.max(0, (Date.parse(`${date}T00:00:00Z`) - Date.parse(`${m.from}T00:00:00Z`)) / 86_400_000)));
  const open = (id: number) => (selectedIssueId.value = id);
  const sentence = $derived(m.cycle_time.p85_days === null ? '' : `85% finish within ${plural(m.cycle_time.p85_days, 'day')}.`);
</script>

{#if pts.length === 0}
  <p class="quiet">No finished issue started and finished in this range.</p>
{:else}
  <figure use:fit={(x) => (width = x)}>
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} role="img" aria-label={`Cycle time of ${pts.length} finished issues. ${sentence}`}>
      {#each ticks as t, i (t)}
        <line class={i === 0 ? 'base' : 'grid'} x1={box.left} x2={w - box.right} y1={s.y(t)} y2={s.y(t)} />
        <text class="tick" x={box.left - 6} y={s.y(t) + 4} text-anchor="end">{t}</text>
      {/each}
      {#if m.cycle_time.p50_days !== null}
        <line class="p50" x1={box.left} x2={w - box.right} y1={s.y(m.cycle_time.p50_days)} y2={s.y(m.cycle_time.p50_days)} />
        <text class="line-label" x={w - box.right} y={s.y(m.cycle_time.p50_days) - 4} text-anchor="end">50%</text>
      {/if}
      {#if m.cycle_time.p85_days !== null}
        <line class="p85" x1={box.left} x2={w - box.right} y1={s.y(m.cycle_time.p85_days)} y2={s.y(m.cycle_time.p85_days)} />
        <text class="line-label" x={w - box.right} y={s.y(m.cycle_time.p85_days) - 4} text-anchor="end">85%</text>
      {/if}
      {#each pts as p (p.issue_id)}
        <circle
          class="dot"
          data-issue={p.issue_id}
          cx={dayX(p.completed)}
          cy={s.y(p.days)}
          r="4"
          role="button"
          tabindex="0"
          aria-label={`${p.key} ${p.title}: ${plural(p.days, 'day')}, finished ${p.completed.slice(5)}`}
          onclick={() => open(p.issue_id)}
          onkeydown={(e) => e.key === 'Enter' && open(p.issue_id)}
        />
      {/each}
    </svg>
    <figcaption>{sentence}</figcaption>
  </figure>
{/if}
{#if m.thin}<p class="thin">Fewer than 5 issues finished in this range, so the times are rough.</p>{/if}

<style>
  figure { margin: 0; display: grid; gap: var(--s-4); min-width: 0; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--border); stroke-width: 1; }
  .base { stroke: var(--control); stroke-width: 1.5; }
  .tick, .line-label { fill: var(--muted); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .p50 { stroke: var(--muted); stroke-width: 1; stroke-dasharray: 4 4; }
  .p85 { stroke: var(--accent); stroke-width: 1.5; }
  .line-label { font-weight: 600; }
  .dot { fill: var(--accent); fill-opacity: 0.75; cursor: pointer; }
  .dot:focus-visible { outline: 2px solid var(--focus, var(--accent)); outline-offset: 2px; }
  figcaption { color: var(--ink); font-size: var(--fs-sm); font-weight: 600; }
  .quiet, .thin { margin: 0; color: var(--muted); font-size: var(--fs-sm); }
  .thin { margin-top: var(--s-5); }
</style>

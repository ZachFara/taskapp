<script lang="ts">
  import { onMount } from 'svelte';
  import { line, scales, type Day } from '../charts/geometry';
  import { chartsOn, defaultMeasure } from './estimation';
  import { progressToToday, today, trim } from './series';

  // A project row's progress: the percent and two tiny lines, scope (grey) and completed (accent), over the
  // project's life. One call per row, because the api has no batched progress query.
  let { projectId, name, pct }: { projectId: number; name: string; pct: number } = $props();
  const W = 56, H = 18;
  let days = $state<Day[]>([]);

  onMount(async () => {
    if (!(await chartsOn())) return;
    const day = await today();
    const p = await progressToToday({ Project: projectId }, await defaultMeasure(), 'auto', day);
    days = p ? trim(p.days) : [];
  });
  const paths = $derived.by(() => {
    const top = Math.max(1, ...days.map((d) => d.scope));
    const s = scales(days.length, top, { w: W, h: H, left: 2, right: 2, top: 2, bottom: 2 });
    const of = (k: 'scope' | 'completed') => line(days.map((d, i) => [s.x(i), s.y(d[k])]));
    return { scope: of('scope'), completed: of('completed') };
  });
</script>

<span class="pspark">
  <span class="pct">{pct}%</span>
  {#if days.length >= 2}
    <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`${name}: scope and completed over time`}>
      <path class="scope" d={paths.scope} />
      <path class="completed" d={paths.completed} />
    </svg>
  {/if}
</span>

<style>
  .pspark { display: inline-flex; align-items: center; gap: var(--s-4); }
  .pct { min-width: 4ch; font-variant-numeric: tabular-nums; }
  svg { display: block; flex: none; }
  path { fill: none; stroke-width: 1.5; stroke-linejoin: round; }
  .scope { stroke: var(--muted); }
  .completed { stroke: var(--accent); }
</style>

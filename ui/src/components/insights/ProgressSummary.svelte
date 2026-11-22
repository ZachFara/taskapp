<script lang="ts">
  import { onMount } from 'svelte';
  import ProgressChart from '../charts/ProgressChart.svelte';
  import type { Day } from '../charts/geometry';
  import Button from '../../ui/Button.svelte';
  import { chartsOn, defaultMeasure, formatFor, unitOf } from './estimation';
  import { insightsApi, type Measure } from './insightsApi';
  import { bounds, today, trim } from './series';

  // The project hero's progress: a compact burn-up when the history has a shape, otherwise the plain bar.
  // `progress` is [done, total] issues, so the bar and the figure keep working without any history.
  let { projectId, progress }: { projectId: number; progress: [number, number] } = $props();

  let days = $state<Day[]>([]);
  let measure = $state<Measure>('count');
  let now = $state('');
  const pct = $derived(progress[1] ? (progress[0] / progress[1]) * 100 : 0);
  const charted = $derived(days.length >= 2 && days.some((d) => d.scope > 0));

  onMount(async () => {
    if (!(await chartsOn())) return;
    measure = await defaultMeasure();
    now = await today();
    const p = await insightsApi.progress({ Project: projectId }, measure, bounds('auto', now).from, bounds('auto', now).to);
    days = p ? trim(p.days) : [];
  });
</script>

<span class="fig">{progress[0]} / {progress[1]}</span>
{#if charted}
  <ProgressChart {days} {measure} unit={unitOf(measure)} today={now} compact format={formatFor(measure)} />
  <span class="open"><Button icon="target" onclick={() => dispatchEvent(new CustomEvent('command:insights.toggle'))}>Insights</Button></span>
{:else}
  <span class="bar" role="img" aria-label={`${progress[0]} of ${progress[1]} done`}><span style:width={`${pct}%`}></span></span>
{/if}

<style>
  .fig { font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .open { display: flex; }
  .bar { display: block; height: 6px; border-radius: var(--r-round); background: var(--band-3); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); }
</style>

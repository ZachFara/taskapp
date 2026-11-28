<script lang="ts">
  import BarChart from '../charts/BarChart.svelte';
  import type { CardProps } from './cards';

  // Work in progress now, and at the end of each period; Little's Law turns the averages into a cycle time.
  let { metrics: m }: CardProps = $props();
  const UNIT = { Day: 'day', Week: 'week', Month: 'month' };
  const unit = $derived(UNIT[m.bucket]);
  const r1 = (n: number) => String(Math.round(n * 10) / 10);
  const plural = (n: number) => `${r1(n)} ${unit}${n === 1 ? '' : 's'}`;
  const avgWip = $derived(m.buckets.reduce((n, b) => n + b.wip, 0) / Math.max(1, m.buckets.length));
  const throughput = $derived(m.buckets.reduce((n, b) => n + b.completed, 0) / Math.max(1, m.buckets.length));
  const law = $derived(
    throughput > 0
      ? `Little's Law: average cycle time = WIP / throughput. Here ${r1(avgWip)} in progress ÷ ${r1(throughput)} finished per ${unit} = ${plural(avgWip / throughput)}.`
      : "Little's Law: average cycle time = WIP / throughput. Nothing finished in these periods, so there is no average.",
  );
</script>

<dl class="now">
  <div><dt>In progress now</dt><dd>{m.wip_now}</dd></div>
</dl>
<BarChart bars={m.buckets.map((b) => ({ label: b.start.slice(5), value: b.wip }))} unit="issues in progress" title="Work in progress by period" />
<p class="law">{law}</p>

<style>
  .now { margin: 0 0 var(--s-5); }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .law { margin: var(--s-5) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

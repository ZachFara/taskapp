<script lang="ts">
  import type { CardProps } from './cards';

  let { metrics: m, measure }: CardProps = $props();
  const done = $derived(m.buckets.reduce((n, b) => n + (measure === 'Points' ? b.completed_points : b.completed), 0));
  const created = $derived(m.buckets.reduce((n, b) => n + b.created, 0));
  const days = (n: number | null) => (n === null ? '–' : `${Math.round(n * 10) / 10} d`);
  const stats = $derived([
    { k: measure === 'Points' ? 'Points completed' : 'Completed', v: String(Math.round(done * 10) / 10) },
    { k: 'Created', v: String(created) },
    { k: 'In progress now', v: String(m.wip_now) },
  ]);
  const times = $derived([
    { k: 'Lead time', p: m.lead_time },
    { k: 'Cycle time', p: m.cycle_time },
  ]);
</script>

<dl class="stats">
  {#each stats as s (s.k)}<div><dt>{s.k}</dt><dd>{s.v}</dd></div>{/each}
</dl>
<table class="times">
  <thead><tr><th scope="col">Time</th><th scope="col">Median</th><th scope="col">85th percentile</th><th scope="col">Issues</th></tr></thead>
  <tbody>
    {#each times as t (t.k)}<tr><th scope="row">{t.k}</th><td>{days(t.p.p50_days)}</td><td>{days(t.p.p85_days)}</td><td>{t.p.n}</td></tr>{/each}
  </tbody>
</table>
{#if m.thin}<p class="thin">Fewer than 5 issues finished in this range, so the times are rough.</p>{/if}

<style>
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--s-5); margin: 0 0 var(--s-6); }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .times { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
  .times th, .times td { text-align: left; padding: var(--s-2) var(--s-3) var(--s-2) 0; font-size: var(--fs-sm); }
  .times thead th { color: var(--muted); font-weight: 500; }
  .times tbody th { font-weight: 600; }
  .thin { margin: var(--s-5) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

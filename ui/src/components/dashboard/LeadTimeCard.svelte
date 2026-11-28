<script lang="ts">
  import type { CardProps } from './cards';

  // Lead time: created to finished, for the issues that finished in the range.
  let { metrics: m }: CardProps = $props();
  const days = (n: number | null) => (n === null ? '–' : `${Math.round(n * 10) / 10} days`);
  const stats = $derived([
    { k: 'Median', v: days(m.lead_time.p50_days) },
    { k: '85th percentile', v: days(m.lead_time.p85_days) },
    { k: 'Issues', v: String(m.lead_time.n) },
  ]);
</script>

<dl class="stats">
  {#each stats as s (s.k)}<div><dt>{s.k}</dt><dd>{s.v}</dd></div>{/each}
</dl>
{#if m.thin}<p class="thin">Fewer than 5 issues finished in this range, so the times are rough.</p>{/if}

<style>
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--s-5); margin: 0; }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .thin { margin: var(--s-5) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

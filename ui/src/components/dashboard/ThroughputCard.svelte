<script lang="ts">
  import BarChart from '../charts/BarChart.svelte';
  import type { CardProps } from './cards';

  // Completed per bucket: counts, or points when the Points switch is on.
  let { metrics: m, measure }: CardProps = $props();
  const bars = $derived(m.buckets.map((b) => ({ label: b.start.slice(5), value: measure === 'Points' ? b.completed_points : b.completed })));
</script>

<BarChart {bars} unit={measure === 'Points' ? 'points' : 'issues'} title={`Completed per ${m.bucket.toLowerCase()}`} />
{#if m.thin}<p class="note">Fewer than 5 issues finished in this range, so these are rough.</p>{/if}

<style>
  .note { margin: var(--s-4) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

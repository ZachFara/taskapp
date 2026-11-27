<script lang="ts">
  import BarChart from '../charts/BarChart.svelte';
  import type { CardProps } from './cards';

  // The last five cycles; BarChart's dashed line is the average of exactly these bars.
  let { metrics: m, measure }: CardProps = $props();
  const last = $derived(m.velocity.slice(-5));
  const bars = $derived(last.map((c) => ({ label: c.name, value: measure === 'Points' ? c.points : c.count })));
</script>

<BarChart {bars} unit={measure === 'Points' ? 'points' : 'issues'} title="Completed per cycle" />
{#if bars.length > 0}
  <p class="note">Dashed line: average of the last {bars.length} cycles.</p>
{/if}
{#if m.velocity.length < 3}<p class="note">Fewer than 3 finished cycles, so the average covers too few to trust.</p>{/if}

<style>
  .note { margin: var(--s-4) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

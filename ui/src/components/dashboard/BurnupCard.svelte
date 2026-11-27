<script lang="ts">
  import ProgressChart from '../charts/ProgressChart.svelte';
  import type { CardProps } from './cards';
  import { burnupDays } from './metrics';

  // Counts, whatever the measure: the engine's scope and done_total are issue counts.
  let { metrics: m }: CardProps = $props();
</script>

<ProgressChart days={burnupDays(m)} measure="count" unit="issues" today={m.to} compact />

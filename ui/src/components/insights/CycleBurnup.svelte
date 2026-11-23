<script lang="ts">
  import { onMount } from 'svelte';
  import ProgressChart from '../charts/ProgressChart.svelte';
  import type { Day } from '../charts/geometry';
  import { defaultMeasure, formatFor, unitOf } from './estimation';
  import { insightsApi, type Measure } from './insightsApi';
  import { today, trim } from './series';

  // The current cycle's burn-up with the ideal pace from its first day to its last.
  let { cycleId }: { cycleId: number } = $props();
  let days = $state<Day[] | null>(null);
  let measure = $state<Measure>('count');
  let now = $state('');

  onMount(async () => {
    measure = await defaultMeasure();
    now = await today();
    const p = await insightsApi.progress({ Cycle: cycleId }, measure);
    days = p ? trim(p.days) : [];
  });
</script>

<ProgressChart {days} {measure} unit={unitOf(measure)} today={now} ideal format={formatFor(measure)} />

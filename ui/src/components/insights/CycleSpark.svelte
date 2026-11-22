<script lang="ts">
  import { onMount } from 'svelte';
  import Sparkline from '../charts/Sparkline.svelte';
  import { defaultMeasure } from './estimation';
  import { insightsApi } from './insightsApi';

  // What one past cycle completed, day by day, as a word-sized line on its row.
  let { cycleId, name }: { cycleId: number; name: string } = $props();
  let values = $state<number[]>([]);

  onMount(async () => {
    const p = await insightsApi.progress({ Cycle: cycleId }, await defaultMeasure());
    values = p?.days.map((d) => d.completed) ?? [];
  });
</script>

<Sparkline {values} label={`${name}: completed over time`} />

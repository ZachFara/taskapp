<script lang="ts">
  import { onMount } from 'svelte';
  import Sparkline from '../charts/Sparkline.svelte';
  import { defaultMeasure } from './estimation';
  import { insightsApi } from './insightsApi';
  import { bounds, today } from './series';

  // What a project completed over the last 30 days, as a tiny line on its sidebar row. One call per row, because
  // the api has no batched progress query.
  let { projectId, name }: { projectId: number; name: string } = $props();
  let values = $state<number[]>([]);

  onMount(async () => {
    const b = bounds('30', await today());
    const p = await insightsApi.progress({ Project: projectId }, await defaultMeasure(), b.from, b.to);
    values = p?.days.map((d) => d.completed) ?? [];
  });
</script>

<span class="spark"><Sparkline {values} label={`${name}: completed over the last 30 days`} w={44} h={16} /></span>

<style>
  /* The row sits on the dark frame, so the line takes the frame's ink instead of the page accent. */
  .spark { --accent: var(--frame-ink); display: flex; margin-left: auto; opacity: 0.75; }
</style>

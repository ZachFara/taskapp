<script lang="ts">
  import type { CardProps } from './cards';
  import ForecastLine from './ForecastLine.svelte';

  // The one card that asks for its own numbers: a forecast is about open work, not the metrics' date range.
  let { filter, cycle }: CardProps = $props();
</script>

<ForecastLine {filter} {cycle} />

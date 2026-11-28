<script lang="ts">
  import { api } from '../../lib/api';
  import type { Filter, Forecast } from '../../lib/bindings';
  import { humanDate } from '../detail/format';

  let { filter = null, cycle = null }: { filter?: Filter | null; cycle?: number | null } = $props();
  let f = $state<Forecast | null>(null);
  let error = $state('');

  // A stale answer (the filter changed while it was loading) is dropped.
  let asked = 0;
  $effect(() => {
    const n = ++asked;
    f = null;
    error = '';
    api.forecast.for({ filter, cycle }).then(
      (r) => n === asked && (f = r),
      (e) => n === asked && (error = (e as { message?: string }).message ?? String(e)),
    );
  });
</script>

{#if error}
  <p role="alert" class="line err">{error}</p>
{:else if f === null}
  <p class="line" aria-busy="true">Forecasting…</p>
{:else if f.open === 0}
  <p class="line">Nothing is open.</p>
{:else if f.p50 && f.p85}
  <p class="line">At the current pace: likely by {humanDate(f.p50)}, 85% by {humanDate(f.p85)}.</p>
{:else}
  <p class="line">Not enough history to forecast: {f.completions} finished in the last {f.window_days} days, 10 are needed.</p>
{/if}

<style>
  .line { margin: 0; color: var(--muted); font-size: var(--fs-sm); }
  .err { color: var(--c-red); }
</style>

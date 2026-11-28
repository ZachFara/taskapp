<script lang="ts">
  import EmptyChart from '../charts/EmptyChart.svelte';

  // One row per group, bars scaled to the largest. Plain text rows, so the numbers read out as they are.
  let { rows, label }: { rows: { label: string; value: number }[]; label: string } = $props();
  const top = $derived(Math.max(0, ...rows.map((r) => r.value)));
  const fmt = (n: number) => String(Math.round(n * 10) / 10);
</script>

{#if top === 0}
  <EmptyChart />
{:else}
  <ul class="rows" aria-label={label}>
    {#each rows as r, i (i)}
      <li><span class="name">{r.label}</span> <span class="track"><span class="fill" style:width={`${(r.value / top) * 100}%`}></span></span> <span class="num">{fmt(r.value)}</span></li>
    {/each}
  </ul>
{/if}

<style>
  .rows { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-3); }
  li { display: grid; grid-template-columns: minmax(7rem, max-content) 1fr auto; align-items: center; gap: var(--s-4); font-size: var(--fs-sm); }
  .track { display: block; height: var(--s-3); background: var(--border); border-radius: var(--r-sm); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
  .num { font-variant-numeric: tabular-nums; font-weight: 600; }
</style>

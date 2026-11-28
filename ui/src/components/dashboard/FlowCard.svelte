<script lang="ts">
  import BarChart from '../charts/BarChart.svelte';
  import type { CardProps } from './cards';
  import { days, pct } from './metrics';

  let { metrics: m }: CardProps = $props();
  const f = $derived(m.flow);
  const labels = ['0–20%', '20–40%', '40–60%', '60–80%', '80–100%'];
</script>

{#if !f || f.n === 0}
  <p class="note">No finished issue here has a cycle time yet, so flow efficiency is blank.</p>
{:else}
  <dl class="stats">
    <div><dt>Average flow efficiency</dt><dd>{f.waiting.length > 0 ? pct(f.avg_pct) : '–'}</dd></div>
    <div><dt>Issues</dt><dd>{f.n}</dd></div>
  </dl>
  {#if f.waiting.length > 0}<BarChart bars={f.distribution.map((value, i) => ({ label: labels[i], value }))} unit="issues" title="Issues by flow efficiency" />{/if}
  {#if f.waiting.length > 0}
    <table>
      <thead><tr><th scope="col">Waiting status</th><th scope="col">Days held</th><th scope="col">Per issue</th></tr></thead>
      <tbody>
        {#each f.waiting as w (w.status_id)}<tr><th scope="row">{w.name}</th><td>{days(w.days)}</td><td>{days(w.days / w.issues)}</td></tr>{/each}
      </tbody>
    </table>
  {:else}
    <p class="note">No time sat in a Waiting status. Mark statuses as Waiting in Settings to see where work stalls.</p>
  {/if}
  {#if f.n < 5}<p class="note">Fewer than 5 issues, so this is rough.</p>{/if}
  <p class="note">Active time over the days from first start to finish, using each status's flow kind.</p>
{/if}

<style>
  .stats { display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--s-5); margin: 0 0 var(--s-5); }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; margin-top: var(--s-5); }
  th, td { text-align: left; padding: var(--s-2) var(--s-3) var(--s-2) 0; font-size: var(--fs-sm); }
  thead th { color: var(--muted); font-weight: 500; }
  tbody th { font-weight: 600; }
  .note { margin: var(--s-4) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

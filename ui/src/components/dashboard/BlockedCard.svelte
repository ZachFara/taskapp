<script lang="ts">
  import type { CardProps } from './cards';
  import { days } from './metrics';

  let { metrics: m }: CardProps = $props();
  const b = $derived(m.blocked);
</script>

{#if !b}
  <p class="note">No blocked-work numbers for this range.</p>
{:else}
  <dl class="stats">
    <div><dt>Blocked now</dt><dd>{b.now.length}</dd></div>
    <div><dt>Blocked time per finished issue</dt><dd>{days(b.avg_days)}</dd></div>
  </dl>
  {#if b.now.length === 0}
    <p class="note">Nothing is waiting on an unfinished blocker.</p>
  {:else}
    <table>
      <thead><tr><th scope="col">Issue</th><th scope="col">Blocked by</th><th scope="col">Days</th></tr></thead>
      <tbody>
        {#each b.now as i (i.issue_id)}<tr><th scope="row"><span class="key">{i.key}</span> {i.title}</th><td>{i.blocked_by.join(', ')}</td><td class="days">{i.days} d</td></tr>{/each}
      </tbody>
    </table>
  {/if}
  {#if b.time.length > 0}
    <h3>Finished in this range, longest blocked</h3>
    <table>
      <tbody>
        {#each b.time.slice(0, 5) as t (t.issue_id)}<tr><th scope="row"><span class="key">{t.key}</span> {t.title}</th><td class="days">{t.days} d</td></tr>{/each}
      </tbody>
    </table>
  {/if}
  <p class="note">Blocked time is approximate: whole days, from an issue's creation to its finish, that ended with a linked blocker still unfinished.</p>
{/if}

<style>
  .stats { display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--s-5); margin: 0 0 var(--s-5); }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  h3 { margin: var(--s-5) 0 var(--s-2); font-size: var(--fs-md); font-weight: 600; }
  table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
  th, td { text-align: left; padding: var(--s-2) var(--s-3) var(--s-2) 0; font-size: var(--fs-sm); }
  thead th { color: var(--muted); font-weight: 500; }
  tbody th { font-weight: 500; }
  .days { white-space: nowrap; }
  .key { color: var(--muted); }
  .note { margin: var(--s-4) 0 0; color: var(--muted); font-size: var(--fs-sm); }
</style>

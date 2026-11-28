<script lang="ts">
  import { navigate } from '../../app/routes';
  import type { CardProps } from './cards';

  let { stale: rows }: CardProps = $props();
  const byStatus = $derived(
    [...rows.reduce((m, s) => m.set(s.status_name, (m.get(s.status_name) ?? 0) + 1), new Map<string, number>())].sort((a, b) => b[1] - a[1]),
  );
</script>

{#if rows.length === 0}
  <p class="quiet">Nothing is stale. An issue is stale when it has sat in a status longer than that status's nudge days. Set nudges in Settings, Statuses; a status without one never makes issues stale.</p>
{:else}
  <p class="count">{rows.length} {rows.length === 1 ? 'issue' : 'issues'} waiting past their nudge</p>
  <ul class="statuses">
    {#each byStatus as [name, n] (name)}<li><span>{name}</span><span>{n}</span></li>{/each}
  </ul>
  <ol class="oldest">
    {#each rows as s (s.issue.id)}
      <li>
        <a class="key" href={`/issue/${s.issue.key}`} onclick={(e) => (e.preventDefault(), navigate(`/issue/${s.issue.key}`))}>{s.issue.key}</a>
        <span class="title">{s.issue.title}</span>
        <span class="days">{s.days} d in {s.status_name}</span>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .quiet, .count { margin: 0 0 var(--s-4); color: var(--muted); font-size: var(--fs-sm); }
  .statuses { list-style: none; margin: 0 0 var(--s-4); padding: 0; display: grid; gap: var(--s-1); font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
  .statuses li { display: flex; justify-content: space-between; }
  .oldest { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-2); font-size: var(--fs-sm); }
  .oldest li { display: grid; grid-template-columns: auto 1fr auto; gap: var(--s-3); align-items: baseline; min-width: 0; }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; text-decoration: none; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .days { color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
</style>

<script lang="ts">
  import { api } from '../../lib/api';
  import type { Change, Issue } from '../../lib/bindings';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import { dateOf } from './format';

  let { issue, rev }: { issue: Issue; rev: number } = $props();
  let changes = $state<Change[]>([]);
  let collapsed = $state(false);

  $effect(() => {
    rev;
    api.history.listFor('issue', String(issue.id)).then((c) => (changes = c));
  });
  const show = (v: unknown) => (v === null || v === undefined || v === '' ? 'none' : typeof v === 'string' ? v : JSON.stringify(v));
  const name = (f: string) => f.replace(/_id$/, '').replace(/_/g, ' ');
  // Newest day first, entries inside a day newest first.
  const days = $derived.by(() => {
    const by = new Map<string, Change[]>();
    for (const c of [...changes].reverse()) by.set(dateOf(c.at), [...(by.get(dateOf(c.at)) ?? []), c]);
    return [...by.entries()];
  });
</script>

<section aria-label="History">
  <GroupHeader name="Activity" count={changes.length} category="Completed" {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  {#if !collapsed}
    {#each days as [day, list] (day)}
      <h4>{day}</h4>
      <ul>
        {#each list as c (c.id)}
          <li><b>{name(c.field)}</b> <span class="from">{show(c.old_value)}</span> <span aria-hidden="true">to</span> <span class="to">{show(c.new_value)}</span></li>
        {/each}
      </ul>
    {:else}
      <p class="none">No changes yet. Created {dateOf(issue.created_at)}.</p>
    {/each}
  {/if}
</section>

<style>
  h4 { padding: var(--s-4) var(--s-5) var(--s-1); font-size: var(--fs-xs); letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); }
  ul { list-style: none; border-left: 2px solid var(--control); margin-left: var(--s-5); }
  li { padding: var(--s-2) var(--s-5); }
  .from { color: var(--muted); text-decoration: line-through; }
  .to { font-weight: 600; }
  .none { padding: var(--s-5); color: var(--muted); }
</style>

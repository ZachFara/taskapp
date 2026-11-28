<script lang="ts">
  import { api } from '../../lib/api';
  import type { FocusSession, Issue } from '../../lib/bindings';
  import Collapse from '../../ui/Collapse.svelte';
  import SectionHeading from './SectionHeading.svelte';

  let { issue, rev }: { issue: Issue; rev: number } = $props();
  let sessions = $state<FocusSession[]>([]);
  let collapsed = $state(true);

  $effect(() => {
    rev;
    api.focus.log(issue.id, null, null).then((s) => (sessions = s));
  });
  const minutes = (s: FocusSession) => Math.round(((s.ended_at ? Date.parse(s.ended_at) : Date.now()) - Date.parse(s.started_at)) / 60_000);
  const total = $derived(sessions.reduce((n, s) => n + minutes(s), 0));
  const when = (iso: string) => new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
</script>

<section aria-label="Focus log">
  <SectionHeading name="Focus log" count={sessions.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  <Collapse open={!collapsed}>
    {#if sessions.length}
      <p class="total">{total} min in {sessions.length} {sessions.length === 1 ? 'session' : 'sessions'}</p>
      <ul>
        {#each sessions as s (s.id)}
          <li>{when(s.started_at)} · {minutes(s)} min{s.deep ? ' · deep' : ''}{s.ended_at ? '' : ' · running'}</li>
        {/each}
      </ul>
    {:else}
      <p class="quiet">No focus sessions yet. Focus on this issue from its header or the command menu.</p>
    {/if}
  </Collapse>
</section>

<style>
  .total, .quiet { margin: 0 0 var(--s-3); color: var(--muted); font-size: var(--fs-sm); }
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-1); font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
</style>

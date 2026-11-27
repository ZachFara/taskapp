<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Event } from '../../lib/bindings';
  import { localDay } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import ListRow from '../../ui/ListRow.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import { workspaceZone } from '../views/dates';
  import EventForm from './EventForm.svelte';
  import { dayHeading, endDay, startDay, timeSpan } from './format';

  let events = $state<Event[]>([]);
  let today = $state(localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone));
  let loaded = $state(false);
  let creating = $state(false);
  let showPast = $state(false);

  async function load() {
    today = localDay(new Date(), await workspaceZone());
    events = await api.events.list(null, null);
    loaded = true;
  }

  // Upcoming runs oldest first from today; past runs newest first. A multi-day event is upcoming until its last day.
  const upcoming = $derived(events.filter((e) => endDay(e) >= today));
  const past = $derived(events.filter((e) => endDay(e) < today).reverse());
  const byDay = (list: Event[]) => {
    const days = new Map<string, Event[]>();
    for (const e of list) days.set(startDay(e), [...(days.get(startDay(e)) ?? []), e]);
    return [...days];
  };

  // The command menu and keys open the form through the registry's event.
  $effect(() => {
    const open = () => (creating = true);
    addEventListener('command:event.new', open);
    addEventListener('command:view.refresh', load);
    return () => {
      removeEventListener('command:event.new', open);
      removeEventListener('command:view.refresh', load);
    };
  });

  onMount(load);
</script>

{#snippet days(list: Event[])}
  {#each byDay(list) as [day, rows] (day)}
    <section class="group" data-day={day}>
      <GroupHeader name={dayHeading(day)} count={rows.length} tone={day === today ? 3 : 2} />
      <ul class="rows">
        {#each rows as e (e.id)}
          <li data-event={e.id}>
            <ListRow onpick={() => navigate(`/event/${e.id}`)}>
              <EntityIcon icon={e.icon ?? 'icon:calendar'} color={e.color} size={16} />
              <span class="time">{timeSpan(e)}</span>
              <span class="title">{e.title}</span>
              {#if e.place}<span class="meta">{e.place}</span>{/if}
              {#if e.issue_ids.length}<span class="meta count">{e.issue_ids.length} {e.issue_ids.length === 1 ? 'issue' : 'issues'}</span>{/if}
            </ListRow>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
{/snippet}

<PageHeader title="Events" icon="icon:calendar">
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New event</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && events.length === 0}
    <EmptyState title="No events yet" hint="An event is something on a date: a meeting, a launch, a deadline day. Link it to the issues it is about.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={() => (creating = true)}>New event</Button>{/snippet}
    </EmptyState>
  {:else if loaded}
    <h3>Upcoming</h3>
    {#if upcoming.length}{@render days(upcoming)}{:else}<p class="none">Nothing coming up.</p>{/if}
    {#if past.length}
      <h3><button type="button" class="toggle" aria-expanded={showPast} onclick={() => (showPast = !showPast)}>Past ({past.length})</button></h3>
      {#if showPast}{@render days(past)}{/if}
    {/if}
  {/if}
</main>

<EventForm bind:open={creating} onsaved={(e) => navigate(`/event/${e.id}`)} />

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-3); }
  .group { display: grid; }
  .rows { list-style: none; margin: 0; padding: 0; }
  li { border-bottom: 1px solid var(--border); }
  .time, .meta { color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .title { font-weight: 600; }
  .count { margin-left: auto; }
  .none { color: var(--muted); }
  h3 { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin: var(--s-5) 0 0; }
  .toggle { padding: 0; border: 0; background: transparent; color: inherit; font: inherit; letter-spacing: inherit; text-transform: inherit; }
</style>

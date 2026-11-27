<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Event } from '../../lib/bindings';
  import { listCommands, type Ctx } from '../../lib/commands';
  import { localDay } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import ContextMenu from '../../ui/ContextMenu.svelte';
  import ListRow from '../../ui/ListRow.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import { workspaceZone } from '../views/dates';
  import EventForm from './EventForm.svelte';
  import { dayHeading, endDay, startDay, timeSpan } from './format';

  let events = $state<Event[]>([]);
  let today = $state(localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone));
  let loaded = $state(false);
  let creating = $state(false);
  let showPast = $state(false);
  // The event the pointer last hit on the list: right-click and the command menu act on it.
  let target = $state<Event | null>(null);
  let editing = $state(false);
  let menuOpen = $state(false);

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
    const on: Record<string, () => void> = {
      'event.new': () => (creating = true),
      'view.refresh': () => void load(),
      'event.open': () => target && navigate(`/event/${target.id}`),
      'event.edit': () => target && (editing = true),
      'event.delete': () => target && void remove(target),
      // The event page focuses its issue picker when it hears this; it mounts after the navigation, so ask late.
      'event.link': () => {
        const e = target;
        if (!e) return;
        navigate(`/event/${e.id}`);
        setTimeout(() => dispatchEvent(new CustomEvent('command:event.link')), 0);
      },
    };
    for (const [k, f] of Object.entries(on)) addEventListener(`command:${k}`, f);
    return () => Object.entries(on).forEach(([k, f]) => removeEventListener(`command:${k}`, f));
  });

  async function remove(e: Event) {
    await api.events.delete(e.id);
    target = null;
    await load();
  }

  // Right-click picks the event under the pointer; off a row there is nothing to act on, so no menu.
  function aim(e: MouseEvent) {
    const row = (e.target as Element).closest<HTMLElement>('[data-event]');
    const hit = row ? events.find((x) => x.id === Number(row.dataset.event)) : undefined;
    if (!hit) return void e.stopPropagation();
    target = hit;
  }
  // Event actions only, in the order the menu shows them. Each one runs its registry command, as the command menu does.
  const ORDER = ['event.open', 'event.edit', 'event.delete', 'event.link'];
  const ctx = (): Ctx => ({ route: '/events', layout: null, selection: [], focusedIssueId: null, openIssueId: null, api, navigate });
  const items = $derived.by(() => {
    if (!menuOpen || !target) return [];
    const live = listCommands(ctx());
    return ORDER.flatMap((id) => live.filter((c) => c.id === id));
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

<ContextMenu bind:open={menuOpen}>
  {#snippet area()}
    <div class="fill" role="presentation" oncontextmenucapture={aim}>
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
    </div>
  {/snippet}
  {#each items as c (c.id)}
    <MenuItem label={c.label} icon={c.icon} shortcut={c.shortcut} onSelect={() => c.run?.(ctx())} />
  {/each}
</ContextMenu>

<EventForm bind:open={creating} onsaved={(e) => navigate(`/event/${e.id}`)} />
{#if target}<EventForm bind:open={editing} event={target} onsaved={load} />{/if}

<style>
  .fill { min-height: 100%; }
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

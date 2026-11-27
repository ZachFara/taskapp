<script lang="ts">
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Event, Issue } from '../../lib/bindings';
  import { labelFor, shortcutFor } from '../../lib/commands/registry';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import IssuePicker from '../detail/IssuePicker.svelte';
  import SectionHeading from '../detail/SectionHeading.svelte';
  import EventForm from './EventForm.svelte';
  import { dayHeading, endDay, startDay, timeSpan } from './format';

  let { params }: { params: { id?: string } } = $props();
  const id = $derived(Number(params.id));

  // undefined while loading, null when there is no such event.
  let event = $state<Event | null | undefined>(undefined);
  let linked = $state<Issue[]>([]);
  let editing = $state(false);
  let error = $state('');
  let box = $state<HTMLElement>();

  async function load() {
    try {
      const e = await api.events.get(id);
      linked = await Promise.all(e.issue_ids.map((i) => api.issues.get(i)));
      event = e;
    } catch (e) {
      event = null;
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  /** Runs a write, then reloads, showing a refusal instead of dropping it. */
  async function act(write: Promise<unknown>) {
    error = '';
    try {
      await write;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
    await load();
  }

  async function remove() {
    await api.events.delete(id);
    navigate('/events');
  }

  $effect(() => {
    id;
    void load();
  });

  // An undo or redo changed it from outside.
  $effect(() => {
    addEventListener('command:view.refresh', load);
    return () => removeEventListener('command:view.refresh', load);
  });

  // The command menu and keys reach this page through the registry's events.
  $effect(() => {
    const on: [string, () => void][] = [
      ['command:event.edit', () => (editing = true)],
      ['command:event.delete', () => void remove()],
      ['command:event.link', () => box?.querySelector<HTMLElement>('input')?.focus()],
    ];
    on.forEach(([n, f]) => addEventListener(n, f));
    return () => on.forEach(([n, f]) => removeEventListener(n, f));
  });
</script>

{#if event === null}
  <EmptyState title="Event not found" hint={error} />
{:else if event}
  <PageHeader title={event.title} icon={event.icon ?? 'icon:calendar'} color={event.color ?? undefined}>
    {#snippet actions()}
      <Button variant="quiet" icon="edit" tooltip={{ label: labelFor('event.edit') ?? 'Edit event', shortcut: shortcutFor('event.edit') }} onclick={() => (editing = true)}>Edit</Button>
      <Button variant="quiet" icon="trash" tooltip={{ label: labelFor('event.delete') ?? 'Delete event', shortcut: shortcutFor('event.delete') }} onclick={remove}>Delete</Button>
    {/snippet}
  </PageHeader>

  <main class="body">
    <dl>
      <dt>When</dt>
      <dd>{dayHeading(startDay(event))}{endDay(event) !== startDay(event) ? ` to ${dayHeading(endDay(event))}` : ''}, {timeSpan(event)}</dd>
      {#if event.place}<dt>Place</dt><dd>{event.place}</dd>{/if}
      {#if event.notes}<dt>Notes</dt><dd class="notes">{event.notes}</dd>{/if}
    </dl>

    <section aria-label="Issues" bind:this={box}>
      <SectionHeading name="Issues" count={linked.length} />
      {#if !linked.length}<p class="none">No linked issues</p>{/if}
      <ul>
        {#each linked as i (i.id)}
          <li>
            <button type="button" class="go" onclick={() => navigate(`/issue/${i.key}`)}><b>{i.key}</b> {i.title}</button>
            <IconButton icon="close" label="Unlink {i.key}" onclick={() => act(api.events.unlink(event!.id, i.id))} />
          </li>
        {/each}
      </ul>
      <div class="add"><IssuePicker selfId={0} label="Link an issue" pick={(i) => act(api.events.link(event!.id, i.id))} /></div>
    </section>
    {#if error}<p role="alert" class="error">{error}</p>{/if}
  </main>

  <EventForm bind:open={editing} {event} onsaved={load} />
{/if}

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); max-width: 720px; }
  dl { display: grid; grid-template-columns: 6rem 1fr; gap: var(--s-3) var(--s-5); margin: 0; }
  dt { color: var(--muted); }
  dd { margin: 0; }
  .notes { white-space: pre-wrap; }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; height: var(--row-h); padding-right: var(--s-4); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; }
  .go:hover { background: var(--active); }
  .none { padding: var(--s-4) var(--s-5); color: var(--muted); }
  .add { padding: var(--s-4); }
  .error { margin: 0; color: var(--signal); font-weight: 600; }
</style>

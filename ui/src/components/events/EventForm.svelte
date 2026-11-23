<script lang="ts">
  import { api } from '../../lib/api';
  import type { Event } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { endDay, startDay } from './format';

  // Creates an event, or edits `event` when given. `onsaved` gets the stored event.
  let { open = $bindable(false), event, onsaved }: { open?: boolean; event?: Event; onsaved: (e: Event) => void } = $props();

  let title = $state('');
  let date = $state('');
  let endDate = $state('');
  let allDay = $state(true);
  let from = $state('09:00');
  let to = $state('10:00');
  let place = $state('');
  let notes = $state('');
  let error = $state('');

  // Each time the dialog opens it starts from the event (or blank).
  $effect(() => {
    if (!open) return;
    error = '';
    title = event?.title ?? '';
    date = event ? startDay(event) : '';
    endDate = event && endDay(event) !== startDay(event) ? endDay(event) : '';
    allDay = event?.all_day ?? true;
    from = event && !event.all_day ? event.starts_at.slice(11, 16) : '09:00';
    to = event?.ends_at && !event.all_day ? event.ends_at.slice(11, 16) : '10:00';
    place = event?.place ?? '';
    notes = event?.notes ?? '';
  });

  async function save() {
    if (!title.trim() || !date) {
      error = 'A title and a date are needed.';
      return;
    }
    const last = endDate || date;
    const input = {
      title: title.trim(),
      starts_at: `${date}T${allDay ? '00:00' : from}:00`,
      // A timed event always ends on its last day at the "to" time; an all-day one only when it spans days.
      ends_at: allDay ? (endDate ? `${endDate}T00:00:00` : null) : `${last}T${to}:00`,
      all_day: allDay,
      place: place.trim() || null,
      notes,
      icon: event?.icon ?? null,
      color: event?.color ?? null,
      archived: event?.archived ?? false,
    };
    try {
      const saved = event ? await api.events.update(event.id, input) : await api.events.create(input);
      open = false;
      onsaved(saved);
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<Dialog bind:open title={event ? 'Edit event' : 'New event'}>
  {#snippet children()}
    <TextInput bind:value={title} label="Event title" placeholder="Title" />
    <TextInput bind:value={date} label="Date" type="date" />
    <TextInput bind:value={endDate} label="End date (optional)" type="date" />
    <label class="row"><Checkbox bind:checked={allDay} label="All day" /> All day</label>
    {#if !allDay}
      <TextInput bind:value={from} label="From" type="time" />
      <TextInput bind:value={to} label="To" type="time" />
    {/if}
    <TextInput bind:value={place} label="Place" placeholder="Place (optional)" />
    <TextArea bind:value={notes} label="Notes" placeholder="Notes (optional)" rows={3} />
    {#if error}<p role="alert" class="error">{error}</p>{/if}
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" onclick={save}>{event ? 'Save event' : 'Create event'}</Button>
  {/snippet}
</Dialog>

<style>
  .row { display: flex; align-items: center; gap: var(--s-4); }
  .error { margin: 0; color: var(--signal); font-weight: 600; }
</style>

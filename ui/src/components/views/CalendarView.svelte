<script lang="ts">
  import { Calendar, DayGrid, Interaction, TimeGrid } from '@event-calendar/core';
  import '@event-calendar/core/index.css';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Due, Issue, SavedView, WorkBlock } from '../../lib/bindings';

  let { view }: { view: SavedView } = $props();

  const HOUR = 60 * 60_000;
  let blocks = $state<WorkBlock[]>([]);
  let issues = $state<Issue[]>([]);
  let selected = $state<number | null>(null);
  let error = $state('');
  let calendar = $state<any>();

  const message = (e: unknown) => (e as { message?: string })?.message ?? String(e);
  const blockId = (id: string) => Number(id.slice('block-'.length));

  // Timed dues are floating wall-clock strings in the due's own zone, so a DST change cannot shift them.
  function dueWallClock(utc: string, tz: string) {
    try {
      const p = Object.fromEntries(
        new Intl.DateTimeFormat('en-CA', {
          timeZone: tz, year: 'numeric', month: '2-digit', day: '2-digit',
          hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23',
        }).formatToParts(new Date(utc)).map((x) => [x.type, x.value]),
      );
      return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}:${p.second}`;
    } catch {
      // shortcut: an unknown IANA zone shows in the local zone, upgrade when the backend validates zones
      return new Date(utc).toISOString().slice(0, 19);
    }
  }

  function dueEvents(i: Issue) {
    const due: Due | null = i.due;
    if (!due) return [];
    const title = `${i.key} ${i.title}`;
    if ('Date' in due) {
      return [{ id: `due-${i.id}`, title, start: due.Date, allDay: true, editable: false, classNames: ['due'] }];
    }
    const { utc, tz } = due.DateTime;
    const end = new Date(Date.parse(utc) + 15 * 60_000).toISOString();
    return [{ id: `due-${i.id}`, title, start: dueWallClock(utc, tz), end: dueWallClock(end, tz), editable: false, classNames: ['due'] }];
  }

  // Blocks go in as local Dates: the calendar reads a Date as the wall clock of its own timezone.
  const events = $derived([
    ...blocks.map((b) => {
      const issue = issues.find((i) => i.id === b.issue_id);
      return {
        id: `block-${b.id}`,
        title: issue ? `${issue.key} ${issue.title}` : `Issue ${b.issue_id}`,
        start: new Date(b.start_utc),
        end: new Date(b.end_utc),
        classNames: ['block'],
      };
    }),
    ...issues.flatMap(dueEvents),
  ]);

  // Read `events` before the optional chain: when the calendar is not yet bound, the chain would skip it
  // and the effect would never track it.
  $effect(() => {
    const current = events;
    calendar?.setOption('events', current);
  });

  async function loadBlocks(start: Date, end: Date) {
    try {
      blocks = await api.blocks.between(start.toISOString(), end.toISOString());
    } catch (e) {
      error = message(e);
    }
  }

  async function onMove(info: any) {
    const { event, revert } = info;
    if (!(event.end > event.start)) {
      revert();
      error = 'A block needs a positive length.';
      return;
    }
    const id = blockId(event.id);
    const start = event.start.toISOString();
    const end = event.end.toISOString();
    try {
      await api.blocks.move(id, start, end);
      blocks = blocks.map((b) => (b.id === id ? { ...b, start_utc: start, end_utc: end } : b));
      error = '';
    } catch (e) {
      revert();
      error = message(e);
    }
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const issueId = Number(e.dataTransfer?.getData('text/plain'));
    const hit = calendar?.dateFromPoint(e.clientX, e.clientY);
    if (!issueId || !hit) return;
    const start = hit.date;
    try {
      const block = await api.blocks.create(issueId, start.toISOString(), new Date(start.getTime() + HOUR).toISOString());
      blocks = [...blocks, block];
      error = '';
    } catch (err) {
      error = message(err);
    }
  }

  async function onKeydown(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    if (typing || selected === null || (e.key !== 'Delete' && e.key !== 'Backspace')) return;
    const id = selected;
    try {
      await api.blocks.delete(id);
      blocks = blocks.filter((b) => b.id !== id);
      selected = null;
      error = '';
    } catch (err) {
      error = message(err);
    }
  }

  const options = {
    view: 'timeGridWeek',
    timeZone: 'local',
    editable: true,
    eventStartEditable: true,
    eventDurationEditable: true,
    headerToolbar: { start: 'title', center: '', end: 'today prev,next timeGridDay,timeGridWeek,dayGridMonth' },
    datesSet: (info: { start: Date; end: Date }) => loadBlocks(info.start, info.end),
    eventClick: (info: { event: { id: string } }) => {
      selected = info.event.id.startsWith('block-') ? blockId(info.event.id) : null;
    },
    eventDrop: onMove,
    eventResize: onMove,
  };

  onMount(async () => {
    try {
      issues = await api.views.query(view.def);
    } catch (e) {
      error = message(e);
    }
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="calendar">
  <aside aria-label="Issues to schedule">
    <ul>
      {#each issues as issue (issue.id)}
        <li draggable="true" ondragstart={(e) => e.dataTransfer?.setData('text/plain', String(issue.id))}>
          <span class="key">{issue.key}</span> {issue.title}
        </li>
      {/each}
    </ul>
  </aside>
  <div class="grid" data-calendar ondragover={(e) => e.preventDefault()} ondrop={onDrop} role="presentation">
    <Calendar bind:this={calendar} plugins={[TimeGrid, DayGrid, Interaction]} {options} />
  </div>
</div>
{#if error}
  <p role="alert">{error}</p>
{/if}

<style>
  .calendar { display: grid; grid-template-columns: 16rem 1fr; gap: 1rem; padding: 1rem; }
  aside ul { list-style: none; margin: 0; padding: 0; }
  aside li { padding: 0.25rem 0.5rem; cursor: grab; border-radius: 4px; }
  aside li:hover { background: color-mix(in srgb, currentColor 8%, transparent); }
  .key { font-variant-numeric: tabular-nums; opacity: 0.6; margin-right: 0.25rem; }
  @media (max-width: 640px) { .calendar { grid-template-columns: 1fr; } }
</style>

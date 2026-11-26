<script lang="ts">
  import { Calendar, DayGrid, Interaction, TimeGrid } from '@event-calendar/core';
  import '@event-calendar/core/index.css';
  import { onMount, untrack } from 'svelte';
  import { navigate } from '../../app/routes';
  import { getWeekStart } from '../../lib/theme.svelte';
  import { selection } from '../../lib/selection.svelte';
  import { api } from '../../lib/api';
  import { labelFor, shortcutFor } from '../../lib/commands';
  import type { Due, Event, Issue, SavedView, Status, StatusCategory, WorkBlock } from '../../lib/bindings';
  import FieldValue from '../display/FieldValue.svelte';
  import { emptyCtx, loadCtx, showWeekends, type Ctx } from '../display/types';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import Icon from '../../ui/Icon.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import Tabs from '../../ui/Tabs.svelte';
  import { calendarSpan } from '../events/format';

  let { view }: { view: SavedView } = $props();

  const HOUR = 60 * 60_000;
  const MODE_KEY = 'cairn.calendar.mode';
  const viewOf = (m: string) => (m === 'month' ? 'dayGridMonth' : 'timeGridWeek');
  // shortcut: no storage in a private window, the mode falls back to month
  const readMode = () => {
    try {
      return localStorage.getItem(MODE_KEY) === 'week' ? 'week' : 'month';
    } catch {
      return 'month';
    }
  };

  let blocks = $state<WorkBlock[]>([]);
  let eventRows = $state<Event[]>([]);
  let issues = $state<Issue[]>([]);
  let statuses = $state<Status[]>([]);
  let loaded = $state(false);
  let selected = $state<number | null>(null);
  let error = $state('');
  let range = $state('');
  let mode = $state(readMode());
  let anchor = new Date();
  let calendar = $state<any>();
  let ctx = $state.raw<Ctx>(emptyCtx);
  // The chip shows the title plus up to two displayed fields; a view with none of its own stays title-only.
  const chipFields = $derived(view.def.columns.slice(0, 2));

  const message = (e: unknown) => (e as { message?: string })?.message ?? String(e);
  const blockId = (id: string) => Number(id.slice('block-'.length));
  const pad = (n: number) => String(n).padStart(2, '0');
  // Local wall-clock parts, as the calendar reads them back from a dropped event.
  const ymd = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const hms = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;

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

  // The zone's offset at an instant, then the instant whose wall clock in `tz` reads `wall` (one correction).
  const offsetMs = (utc: number, tz: string) => Date.parse(`${dueWallClock(new Date(utc).toISOString(), tz)}Z`) - utc;
  function wallToUtc(wall: string, tz: string) {
    const asUtc = Date.parse(`${wall}Z`);
    const first = asUtc - offsetMs(asUtc, tz);
    return new Date(asUtc - offsetMs(first, tz)).toISOString();
  }

  const statusOf = (i: Issue) => statuses.find((s) => s.id === i.status_id);
  const categoryOf = (i: Issue): StatusCategory => statusOf(i)?.category ?? 'Backlog';
  const isDue = (id: string) => id.startsWith('due-');

  function dueEvents(i: Issue) {
    const due: Due | null = i.due;
    if (!due) return [];
    const extendedProps = { key: i.key, category: categoryOf(i), icon: statusOf(i)?.icon, color: statusOf(i)?.color, issue: i };
    const title = i.title;
    if ('Date' in due) {
      return [{ id: `due-${i.id}`, title, start: due.Date, allDay: true, editable: true, durationEditable: false, classNames: ['due'], extendedProps }];
    }
    const { utc, tz } = due.DateTime;
    const end = new Date(Date.parse(utc) + 15 * 60_000).toISOString();
    return [{ id: `due-${i.id}`, title, start: dueWallClock(utc, tz), end: dueWallClock(end, tz), editable: true, durationEditable: false, classNames: ['due'], extendedProps }];
  }

  // Blocks go in as local Dates: the calendar reads a Date as the wall clock of its own timezone.
  const events = $derived([
    ...blocks.map((b) => {
      const issue = issues.find((i) => i.id === b.issue_id);
      return {
        id: `block-${b.id}`,
        title: issue ? issue.title : `Issue ${b.issue_id}`,
        start: new Date(b.start_utc),
        end: new Date(b.end_utc),
        classNames: ['block'],
        extendedProps: { key: issue?.key, category: issue ? categoryOf(issue) : 'Backlog', icon: issue ? statusOf(issue)?.icon : null, color: issue ? statusOf(issue)?.color : null, issue },
      };
    }),
    ...issues.flatMap(dueEvents),
    // Events are not issue work: their own chip, opened (not dragged) from here.
    ...eventRows.map((e) => {
      const { start, end, allDay } = calendarSpan(e);
      return { id: `event-${e.id}`, title: e.title, start, end, allDay, editable: false, classNames: ['event'], extendedProps: { isEvent: true } };
    }),
  ]);
  const unscheduled = $derived(issues.filter((i) => !i.due));

  // Read `events` before the optional chain: when the calendar is not yet bound, the chain would skip it
  // and the effect would never track it.
  $effect(() => {
    const current = events;
    calendar?.setOption('events', current);
  });

  $effect(() => {
    const m = mode;
    calendar?.setOption('view', viewOf(m));
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch {
      // shortcut: the mode is not remembered without storage, upgrade with a stored view setting
    }
  });

  // Commands reach the view as window events; the registry decides when their keys fire.
  $effect(() => {
    const actions: Record<string, () => void> = {
      'calendar.prev': () => shift(-1),
      'calendar.next': () => shift(1),
      'calendar.today': () => goTo(new Date()),
      'calendar.week': () => (mode = 'week'),
      'calendar.month': () => (mode = 'month'),
    };
    const listeners = Object.entries(actions).map(([id, run]) => {
      const h = () => run();
      window.addEventListener(`command:${id}`, h);
      return [id, h] as const;
    });
    return () => {
      for (const [id, h] of listeners) window.removeEventListener(`command:${id}`, h);
    };
  });

  const fire = (id: string) => window.dispatchEvent(new CustomEvent(`command:${id}`));

  function shift(dir: number) {
    const d = new Date(anchor);
    if (mode === 'month') d.setMonth(d.getMonth() + dir, 1);
    else d.setDate(d.getDate() + 7 * dir);
    goTo(d);
  }

  function goTo(d: Date) {
    anchor = d;
    calendar?.setOption('date', d);
  }

  async function loadBlocks(start: Date, end: Date) {
    try {
      [blocks, eventRows] = await Promise.all([
        api.blocks.between(start.toISOString(), end.toISOString()),
        api.events.list(ymd(start), ymd(new Date(end.getTime() - 1))),
      ]);
    } catch (e) {
      error = message(e);
    }
  }

  const fmt = (d: Date, o: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(undefined, o).format(d);

  function onRange(start: Date, end: Date) {
    range =
      mode === 'week'
        ? `${fmt(start, { day: 'numeric', month: 'short' })} to ${fmt(new Date(end.getTime() - 1), { day: 'numeric', month: 'short', year: 'numeric' })}`
        : fmt(new Date((start.getTime() + end.getTime()) / 2), { month: 'long', year: 'numeric' });
    return loadBlocks(start, end);
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

  // A due keeps its kind: a date stays a date, a timed due keeps its zone and, when dropped on the all-day row,
  // its time of day.
  async function moveDue(info: any) {
    const { event, revert } = info;
    const id = Number(String(event.id).slice('due-'.length));
    const issue = issues.find((i) => i.id === id);
    if (!issue?.due) {
      revert();
      return;
    }
    const day = ymd(event.start);
    let due: Due;
    if ('Date' in issue.due) {
      due = { Date: day };
    } else {
      const { utc, tz } = issue.due.DateTime;
      const time = event.allDay ? dueWallClock(utc, tz).slice(11) : hms(event.start);
      due = { DateTime: { utc: wallToUtc(`${day}T${time}`, tz), tz } };
    }
    try {
      const updated = await api.issues.update(id, { due });
      issues = issues.map((i) => (i.id === id ? updated : i));
      error = '';
    } catch (e) {
      revert();
      error = message(e);
    }
  }

  // An issue dropped on the all-day row gets that date as its due; dropped on a time slot it becomes a 60-minute block.
  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const issueId = Number(e.dataTransfer?.getData('text/plain'));
    const hit = calendar?.dateFromPoint(e.clientX, e.clientY);
    if (!issueId || !hit) return;
    try {
      if (hit.allDay) {
        const updated = await api.issues.update(issueId, { due: { Date: ymd(hit.date) } });
        issues = issues.map((i) => (i.id === issueId ? updated : i));
      } else {
        const start = hit.date;
        const block = await api.blocks.create(issueId, start.toISOString(), new Date(start.getTime() + HOUR).toISOString());
        blocks = [...blocks, block];
      }
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
    view: viewOf(mode),
    timeZone: 'local',
    firstDay: 0,
    scrollTime: '08:00:00',
    height: '100%',
    // Month rows share the sheet's height and long days show "+ more" instead of growing.
    dayMaxEvents: true,
    // A month header names only the weekday; a week header adds the date.
    views: {
      dayGridMonth: { dayHeaderFormat: (d: Date) => d.toLocaleDateString('en-US', { weekday: 'short' }) },
      timeGridWeek: { dayHeaderFormat: (d: Date) => `${d.toLocaleDateString('en-US', { weekday: 'short' })} ${d.getDate()}` },
    },
    headerToolbar: false,
    hiddenDays: showWeekends(view.def) ? [] : [0, 6],
    editable: true,
    eventStartEditable: true,
    eventDurationEditable: true,
    datesSet: (info: { start: Date; end: Date }) => onRange(info.start, info.end),
    eventClick: (info: { event: { id: string } }) => {
      if (info.event.id.startsWith('event-')) return navigate(`/event/${info.event.id.slice('event-'.length)}`);
      selected = info.event.id.startsWith('block-') ? blockId(info.event.id) : null;
    },
    eventDrop: (info: any) => (isDue(info.event.id) ? moveDue(info) : onMove(info)),
    eventResize: onMove,
  };

  let queryVersion = 0;
  async function loadIssues() {
    const mine = ++queryVersion;
    const def = $state.snapshot(view.def);
    try {
      const result = await Promise.all([api.statuses.list(), api.views.query(def), loadCtx(def)]);
      if (mine !== queryVersion) return;
      [statuses, issues, ctx] = result;
      loaded = true;
    } catch (e) {
      if (mine === queryVersion) error = message(e);
    }
  }
  $effect(() => {
    view.def;
    untrack(loadIssues);
  });
  onMount(async () => {
    selection.setLayout('calendar');
    options.firstDay = (await getWeekStart()) === 'monday' ? 1 : 0;
    calendar?.setOption('firstDay', options.firstDay);
  });

  const modes = [
    { id: 'week', label: 'Week' },
    { id: 'month', label: 'Month' },
  ];
</script>

<svelte:window onkeydown={onKeydown} />

<div class="calendar">
  <div class="toolbar">
    <Tabs bind:value={mode} tabs={modes} label="Calendar range" segmented />
    <span class="range" aria-live="polite">{range}</span>
    <div class="grow"></div>
    <IconButton icon="chevron-left" label={labelFor('calendar.prev') ?? 'Previous'} shortcut={shortcutFor('calendar.prev')} onclick={() => fire('calendar.prev')} />
    <Button variant="quiet" tooltip={{ label: labelFor('calendar.today') ?? 'Today', shortcut: shortcutFor('calendar.today') }} onclick={() => fire('calendar.today')}>Today</Button>
    <IconButton icon="chevron-right" label={labelFor('calendar.next') ?? 'Next'} shortcut={shortcutFor('calendar.next')} onclick={() => fire('calendar.next')} />
  </div>

  <div class="body">
    <div class="grid" data-calendar ondragover={(e) => e.preventDefault()} ondrop={onDrop} role="presentation">
      <Calendar bind:this={calendar} plugins={[TimeGrid, DayGrid, Interaction]} {options}>
        {#snippet eventContent(info: any)}
          {@const x = info.event.extendedProps}
          <span class="chip" data-key={x.key} title={x.key}>
            {#if x.isEvent}<Icon name="calendar" size={12} />{:else if x.category}<StatusGlyph category={x.category} icon={x.icon} color={x.color} size={12} />{/if}
            <span class="text">{info.event.title}</span>
            {#if x.issue}{#each chipFields as f (f.Property ?? f.Fixed)}<span class="fv"><FieldValue field={f} issue={x.issue} {ctx} /></span>{/each}{/if}
          </span>
        {/snippet}
      </Calendar>
    </div>

    <aside class="side" aria-label="Issues to schedule">
      <GroupHeader name="Unscheduled" count={unscheduled.length} category="Backlog" />
      {#if !loaded}
        <div class="loading"><Skeleton /><Skeleton /><Skeleton /></div>
      {:else if unscheduled.length}
        <ul aria-label="Unscheduled">
          {#each unscheduled as issue (issue.id)}
            <li draggable="true" tabindex="0" data-key={issue.key} ondragstart={(e) => e.dataTransfer?.setData('text/plain', String(issue.id))}>
              <StatusGlyph category={categoryOf(issue)} icon={statusOf(issue)?.icon} color={statusOf(issue)?.color} size={14} />
              <span class="key">{issue.key}</span>
              <span class="text">{issue.title}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <EmptyState title="Everything has a date" hint="Issues without a due date land here." />
      {/if}
    </aside>
  </div>
</div>
{#if error}
  <p role="alert" class="alert">{error}</p>
{/if}

<style>
  .calendar { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .toolbar { flex: none; height: var(--topbar-h); display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-7); }
  .grow { flex: 1; }
  .range { font-size: var(--fs-lg); font-weight: 700; letter-spacing: -0.01em; padding: 0 var(--s-6); }
  .body { flex: 1; min-height: 0; display: grid; grid-template-rows: minmax(0, 1fr); grid-template-columns: minmax(0, 1fr) calc(var(--sidebar-w) + var(--s-9)); border-top: 1px solid var(--border); }
  .grid {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: var(--s-6) var(--s-7);
    font-family: var(--font);
    font-size: var(--fs-md);
    color: var(--ink);
  }
  /* The library sets its variables and its font on .ec itself, so they are overridden there. */
  .grid :global(.ec) {
    --ec-bg-color: var(--sheet);
    --ec-day-bg-color: var(--sheet);
    --ec-border-color: var(--border);
    --ec-text-color: var(--ink);
    --ec-today-bg-color: transparent;
    --ec-highlight-color: var(--selection);
    --ec-popup-bg-color: var(--raised);
    --ec-now-indicator-color: var(--signal);
    --ec-event-bg-color: var(--card);
    --ec-event-text-color: var(--ink);
    font-family: inherit;
    font-size: inherit;
    border: 0;
    flex: 1;
    min-height: 0;
  }
  .grid :global(.ec-day-head) { color: var(--muted); font-size: var(--fs-sm); font-weight: 600; }
  .grid :global(.ec-today .ec-day-head) { color: var(--ink); font-weight: 700; background: color-mix(in srgb, var(--signal) 14%, transparent); box-shadow: inset 0 var(--s-1) 0 var(--signal); }
  .grid :global(.ec-today .ec-day-num), .grid :global(.ec-today .ec-day-head) { color: var(--signal); }
  .grid :global(.ec-event) { border: 0; border-left: 3px solid var(--control); border-radius: var(--r-sm); }
  .grid :global(.ec-event.due) { border-left-color: var(--stone); }
  .grid :global(.ec-event.event) { --ec-event-bg-color: var(--active); border-left-color: var(--signal); }
  .grid :global(.ec-event.block) { --ec-event-bg-color: var(--band-3); border-left-color: var(--accent); }
  .fv { flex: none; display: inline-flex; }
  .chip { display: flex; align-items: center; gap: var(--s-3); min-width: 0; padding: var(--s-1) var(--s-3); font-size: var(--fs-sm); }
  .text { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .key { flex: none; white-space: nowrap; font-weight: 700; color: var(--muted); margin-right: var(--s-2); }

  .side { min-width: 0; min-height: 0; display: flex; flex-direction: column; border-left: 1px solid var(--border); background: var(--raised); }
  .side ul { list-style: none; margin: 0; padding: 0; overflow: auto; }
  .side li {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    height: var(--row-h);
    padding: 0 var(--s-7);
    border-bottom: 1px solid var(--border);
    cursor: grab;
    transition: background var(--dur-1) var(--ease);
  }
  .side li:hover { background: var(--active); }
  .side li .text { font-size: var(--fs-md); }
  .loading { display: grid; gap: var(--s-4); padding: var(--s-6) var(--s-7); }
  .alert { margin: 0 var(--s-7); color: var(--signal); font-weight: 600; }

  /* Narrow window: the unscheduled list goes under the grid so the chips keep their width. */
  @media (max-width: 1100px) {
    .body { grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr) auto; }
    .side { border-left: 0; border-top: 1px solid var(--border); max-height: 30vh; }
  }
  @media (max-width: 640px) {
    .body { grid-template-columns: 1fr; }
    .side { border-left: 0; }
  }
</style>

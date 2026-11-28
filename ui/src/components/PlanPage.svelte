<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Event, Issue, Status } from '../lib/bindings';
  import { FIRST, LAST, SLOTS, STEP, addDays, candidates, clock, fit, minuteAt, minutesOf, stamp } from '../lib/plan';
  import { localDay } from '../lib/gantt';
  import Button from '../ui/Button.svelte';
  import CommandButton from './CommandButton.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Select from '../ui/Select.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { dayHeading } from './events/format';
  import { workspaceZone } from './views/dates';

  // Today plan: a day column for today and tomorrow. Dragging an issue onto it makes an event linked to the issue;
  // blocks (events with a linked issue) move, resize and delete. Everything also works from the form and the command menu.
  let tz = $state(Intl.DateTimeFormat().resolvedOptions().timeZone);
  let today = $state(localDay(new Date(), tz));
  let issues = $state<Issue[]>([]);
  let statuses = $state<Status[]>([]);
  let nextIds = $state(new Set<number>());
  let events = $state<Event[]>([]);
  let loaded = $state(false);
  let picked = $state<number | null>(null); // issue chosen in the side list
  let selected = $state<number | null>(null); // event id of the chosen block

  const days = $derived([today, addDays(today, 1)]);
  const groups = $derived(candidates(issues, statuses, nextIds, today, tz));
  const issueOf = (id: number) => issues.find((i) => i.id === id);
  const isDone = (id: number) => issueOf(id)?.completed_at != null;
  const onDay = (day: string) => events.filter((e) => !e.all_day && e.starts_at.slice(0, 10) === day);
  const isBlock = (e: Event) => e.issue_ids.length > 0;

  async function load() {
    tz = await workspaceZone();
    today = localDay(new Date(), tz);
    const noView = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false } as const;
    [issues, statuses, nextIds, events] = await Promise.all([
      api.views.query(noView),
      api.statuses.list(),
      api.next.list().then((n) => new Set(n.flatMap((r) => (r.issue ? [r.issue.id] : [])))),
      api.events.list(today, addDays(today, 1)),
    ]);
    loaded = true;
  }
  onMount(() => {
    void load();
    const refresh = () => void load();
    for (const e of ['rows:changed', 'history:wrote', 'command:view.refresh']) addEventListener(e, refresh);
    return () => ['rows:changed', 'history:wrote', 'command:view.refresh'].forEach((e) => removeEventListener(e, refresh));
  });

  // End of day: minutes planned, and minutes in blocks whose issues are all done.
  const summary = (day: string) => {
    const blocks = onDay(day).filter(isBlock);
    const mins = (list: Event[]) => list.reduce((n, e) => n + (e.ends_at ? minutesOf(e.ends_at) - minutesOf(e.starts_at) : 0), 0);
    const done = blocks.filter((e) => e.issue_ids.every(isDone));
    return { n: blocks.length, planned: mins(blocks), done: mins(done), doneN: done.length };
  };
  const hours = (m: number) => `${Math.floor(m / 60)}h${m % 60 ? ` ${m % 60}m` : ''}`;

  const input = (e: Event, day: string, from: number, to: number) => ({
    title: e.title, starts_at: stamp(day, from), ends_at: stamp(day, to), all_day: false, place: e.place, notes: e.notes, icon: e.icon, color: e.color, archived: e.archived,
  });
  // The estimate, at most 4 hours: a long estimate is worked in several blocks.
  const lengthFor = (i: Issue) => Math.min(240, Math.max(STEP, Math.round((i.estimate_minutes ?? 60) / STEP) * STEP));

  async function create(issue: Issue, day: string, start: number, length: number) {
    const [from, to] = fit(start, length);
    const ev = await api.events.create({ title: `${issue.key} ${issue.title}`, starts_at: stamp(day, from), ends_at: stamp(day, to), all_day: false, place: null, notes: '', icon: null, color: null, archived: false });
    await api.events.link(ev.id, issue.id);
    selected = ev.id;
    await load();
  }
  async function reshape(e: Event, day: string, start: number, length: number) {
    const [from, to] = fit(start, length);
    await api.events.update(e.id, input(e, day, from, to));
    await load();
  }
  async function remove(e: Event) {
    await api.events.delete(e.id);
    selected = null;
    await load();
  }

  // Dragging uses pointer events: the web view in the Mac app does not deliver HTML5 drag events reliably.
  type Drag = { kind: 'new'; issue: Issue; x: number; y: number } | { kind: 'move' | 'resize'; event: Event; grab: number; x: number; y: number };
  let drag = $state<Drag | null>(null);
  let pending: Drag | null = null;
  let ghost = $state<{ x: number; y: number; text: string } | null>(null);

  function begin(d: Drag) {
    pending = d;
    addEventListener('pointermove', move, true);
    addEventListener('pointerup', up, true);
    addEventListener('keydown', key, true);
  }
  function stop() {
    removeEventListener('pointermove', move, true);
    removeEventListener('pointerup', up, true);
    removeEventListener('keydown', key, true);
    pending = drag = ghost = null;
  }
  function move(e: PointerEvent) {
    if (!pending) return;
    if (!drag && Math.hypot(e.clientX - pending.x, e.clientY - pending.y) < 4) return;
    drag = pending;
    ghost = { x: e.clientX, y: e.clientY, text: drag.kind === 'new' ? `${drag.issue.key} ${drag.issue.title}` : drag.event.title };
  }
  const key = (e: KeyboardEvent) => e.key === 'Escape' && (e.preventDefault(), stop());
  function up(e: PointerEvent) {
    const d = drag;
    stop();
    if (!d) return;
    // The click that ends a drag must not select what the drag started on.
    const swallow = (c: Event) => (c.preventDefault(), c.stopPropagation());
    addEventListener('click', swallow, { capture: true, once: true });
    setTimeout(() => removeEventListener('click', swallow, true), 0);
    const col = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-day]');
    if (!col) return;
    const r = col.getBoundingClientRect();
    const day = col.dataset.day!;
    const at = minuteAt(e.clientY - r.top - (d.kind === 'move' ? d.grab : 0), r.height);
    if (d.kind === 'new') void create(d.issue, day, at, lengthFor(d.issue));
    else {
      const from = minutesOf(d.event.starts_at);
      const to = d.event.ends_at ? minutesOf(d.event.ends_at) : from + 60;
      if (d.kind === 'move') void reshape(d.event, day, at, to - from);
      else if (day === d.event.starts_at.slice(0, 10)) void reshape(d.event, day, from, at + STEP - from);
    }
  }
  const pxTop = (m: number) => `calc(${(m - FIRST) / STEP} * var(--plan-slot))`;

  // The form: plan an issue, or change the selected block's start and length.
  let formOpen = $state(false);
  let editing = $state<Event | null>(null);
  let fIssue = $state('');
  let fDay = $state('');
  let fStart = $state('09:00');
  let fLength = $state('60');
  let error = $state('');
  function openForm(block: Event | null) {
    error = '';
    editing = block;
    const issue = picked !== null ? issueOf(picked) : undefined;
    fIssue = String(issue?.id ?? '');
    fDay = block ? block.starts_at.slice(0, 10) : today;
    fStart = block ? block.starts_at.slice(11, 16) : '09:00';
    fLength = block?.ends_at ? String(minutesOf(block.ends_at) - minutesOf(block.starts_at)) : String(issue ? lengthFor(issue) : 60);
    formOpen = true;
  }
  async function save() {
    const [h, m] = fStart.split(':').map(Number);
    const length = Number(fLength);
    const issue = issueOf(Number(fIssue));
    if (!fDay || Number.isNaN(h) || !(length > 0) || (!editing && !issue)) {
      error = 'Pick an issue, a day, a start time and a length in minutes.';
      return;
    }
    try {
      if (editing) await reshape(editing, fDay, h * 60 + m, length);
      else await create(issue!, fDay, h * 60 + m, length);
      formOpen = false;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  $effect(() => {
    const chosen = () => events.find((e) => e.id === selected);
    const on: Record<string, () => void> = {
      'plan.add': () => openForm(null),
      'plan.edit': () => { const b = chosen(); if (b) openForm(b); },
      'plan.remove': () => { const b = chosen(); if (b) void remove(b); },
    };
    for (const [k, f] of Object.entries(on)) addEventListener(`command:${k}`, f);
    return () => Object.entries(on).forEach(([k, f]) => removeEventListener(`command:${k}`, f));
  });

  const choices = $derived(groups.flatMap((g) => g.issues).map((i) => ({ value: String(i.id), label: `${i.key} ${i.title}` })));
  const hourLabels = Array.from({ length: (LAST - FIRST) / 60 }, (_, h) => FIRST + h * 60);
</script>

<PageHeader title="Today plan" icon="icon:clock">
  {#snippet actions()}
    {#if selected !== null}<CommandButton command="plan.edit" /><CommandButton command="plan.remove" />{/if}
    <CommandButton command="plan.add" variant="primary" />
  {/snippet}
</PageHeader>

<main class="body">
  {#if !loaded}
    <p class="muted">Loading…</p>
  {:else}
    <aside class="side" aria-label="Issues to plan">
      {#if groups.length === 0}
        <EmptyState title="Nothing to plan" hint="Next actions, issues due soon and deep work show up here." />
      {/if}
      {#each groups as g (g.label)}
        <h3>{g.label}</h3>
        <ul>
          {#each g.issues as i (i.id)}
            <li>
              <button class="cand" class:on={picked === i.id} aria-pressed={picked === i.id} data-issue={i.id} onpointerdown={(e) => e.button === 0 && begin({ kind: 'new', issue: i, x: e.clientX, y: e.clientY })} onclick={() => (picked = picked === i.id ? null : i.id)}>
                <span class="key">{i.key}</span> <span class="t">{i.title}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/each}
    </aside>

    {#each days as day, n (day)}
      {@const s = summary(day)}
      <section class="day" aria-label={dayHeading(day)}>
        <h2>{n === 0 ? 'Today' : 'Tomorrow'} <span class="muted">{dayHeading(day)}</span></h2>
        <div class="col" data-day={day} class:over={drag !== null} style:height={`calc(${SLOTS} * var(--plan-slot))`}>
          {#each hourLabels as m (m)}<div class="hour" style:top={pxTop(m)}><span>{clock(m)}</span></div>{/each}
          {#each onDay(day) as e (e.id)}
            {@const from = Math.max(FIRST, minutesOf(e.starts_at))}
            {@const to = Math.min(LAST, e.ends_at ? minutesOf(e.ends_at) : from + 60)}
            {#if isBlock(e)}
              <div class="block" class:sel={selected === e.id} class:done={e.issue_ids.every(isDone)} data-event={e.id} role="button" tabindex="0" aria-label={`${e.title}, ${clock(from)} to ${clock(to)}`}
                style:top={pxTop(from)} style:height={`calc(${Math.max(1, (to - from) / STEP)} * var(--plan-slot))`}
                onpointerdown={(ev) => ev.button === 0 && begin({ kind: 'move', event: e, grab: ev.clientY - ev.currentTarget.getBoundingClientRect().top, x: ev.clientX, y: ev.clientY })}
                onclick={() => (selected = e.id)} onkeydown={(ev) => ev.key === 'Enter' && (selected = e.id)}>
                <span class="when">{clock(from)}</span> <span class="t">{e.title}</span>
                <button class="x" aria-label={`Remove ${e.title}`} onpointerdown={(ev) => ev.stopPropagation()} onclick={(ev) => (ev.stopPropagation(), void remove(e))}>×</button>
                <div class="grip" role="separator" aria-label={`Resize ${e.title}`} onpointerdown={(ev) => (ev.stopPropagation(), ev.button === 0 && begin({ kind: 'resize', event: e, grab: 0, x: ev.clientX, y: ev.clientY }))}></div>
              </div>
            {:else}
              <div class="fixed" style:top={pxTop(from)} style:height={`calc(${Math.max(1, (to - from) / STEP)} * var(--plan-slot))`}><span class="t">{e.title}</span></div>
            {/if}
          {/each}
        </div>
        <p class="sum" data-summary={day}>Planned {hours(s.planned)} in {s.n} {s.n === 1 ? 'block' : 'blocks'}; done {hours(s.done)} ({s.doneN} of {s.n}).</p>
      </section>
    {/each}
  {/if}
</main>

{#if ghost}<div class="ghost" style:transform={`translate(${ghost.x + 12}px, ${ghost.y + 8}px)`}>{ghost.text}</div>{/if}

<Dialog bind:open={formOpen} title={editing ? 'Change time block' : 'Time-block an issue'}>
  {#snippet children()}
    <div class="fields">
      {#if !editing}<label class="field"><span>Issue</span><Select bind:value={fIssue} items={choices} label="Issue" placeholder="Pick an issue" /></label>{/if}
      <label class="field"><span>Day</span><TextInput bind:value={fDay} label="Day" type="date" /></label>
      <label class="field"><span>Start</span><TextInput bind:value={fStart} label="Start" type="time" step="900" /></label>
      <label class="field"><span>Length in minutes</span><TextInput bind:value={fLength} label="Length in minutes" type="number" step="15" min="15" /></label>
      {#if error}<p role="alert" class="error">{error}</p>{/if}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (formOpen = false)}>Cancel</Button>
    <Button variant="primary" onclick={save}>{editing ? 'Save block' : 'Add block'}</Button>
  {/snippet}
</Dialog>

<style>
  .body { display: grid; grid-template-columns: minmax(200px, 280px) repeat(2, minmax(220px, 1fr)); gap: var(--s-7); padding: 0 var(--s-7) var(--s-7); align-items: start; }
  .muted { color: var(--muted); font-weight: 400; }
  h2 { font-size: var(--fs-lg); font-weight: 700; margin: 0 0 var(--s-4); }
  h3 { font-size: var(--fs-xs); font-weight: 600; color: var(--muted); margin: var(--s-6) 0 var(--s-3); }
  ul { list-style: none; margin: 0; padding: 0; }
  .cand { display: flex; gap: var(--s-4); width: 100%; min-height: var(--row-h); align-items: center; padding: 0 var(--s-4); border: 1px solid transparent; border-radius: var(--r-ctl); background: none; color: inherit; text-align: left; cursor: grab; touch-action: none; }
  .cand:hover, .cand.on { background: var(--active); }
  .cand.on { border-color: var(--border); }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; }
  .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .col { position: relative; border: 1px solid var(--border); border-radius: var(--r-ctl); user-select: none; }
  .col.over { background: var(--active); }
  .hour { position: absolute; left: 0; right: 0; height: calc(4 * var(--plan-slot)); border-top: 1px solid var(--border); pointer-events: none; }
  .hour span { position: absolute; left: var(--s-3); top: var(--s-1); font-size: var(--fs-xs); color: var(--muted); font-variant-numeric: tabular-nums; }
  .block, .fixed { position: absolute; left: 48px; right: var(--s-3); box-sizing: border-box; overflow: hidden; padding: 0 var(--s-4); border-radius: var(--r-ctl); font-size: var(--fs-sm); line-height: calc(2 * var(--plan-slot)); }
  .block { display: flex; gap: var(--s-3); background: var(--accent); color: var(--accent-ink); cursor: grab; touch-action: none; }
  .block.sel { outline: 2px solid var(--focus); outline-offset: 1px; }
  .block.done { opacity: 0.6; }
  .block .when { font-variant-numeric: tabular-nums; opacity: 0.8; }
  .block .t { flex: 1; }
  .x { align-self: flex-start; border: 0; background: none; color: inherit; cursor: pointer; padding: 0 var(--s-2); font-size: var(--fs-lg); line-height: inherit; }
  .grip { position: absolute; left: 0; right: 0; bottom: 0; height: var(--s-3); cursor: ns-resize; background: color-mix(in srgb, currentColor 35%, transparent); }
  .fixed { background: var(--control); color: var(--ink); }
  .sum { margin: var(--s-4) 0 0; color: var(--muted); font-size: var(--fs-sm); }
  .ghost { position: fixed; left: 0; top: 0; z-index: var(--z-tooltip); pointer-events: none; padding: var(--s-2) var(--s-5); border-radius: var(--r-ctl); background: var(--frame-raised); color: var(--frame-ink); font-size: var(--fs-md); font-weight: 500; box-shadow: var(--shadow-float); }
  .fields { display: grid; gap: var(--s-4); }
  .field { display: grid; gap: var(--s-2); font-weight: 600; }
  .error { margin: 0; color: var(--signal); font-weight: 600; }
</style>

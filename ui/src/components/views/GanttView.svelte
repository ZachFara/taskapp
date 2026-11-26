<script lang="ts">
  import FieldValue from '../display/FieldValue.svelte';
  import { emptyCtx, loadCtx, type Ctx } from '../display/types';
  import { untrack } from 'svelte';
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Issue, IssuePatch, Milestone, Project, Relation, Reschedule, SavedView, Status, ViewDef } from '../../lib/bindings';
  import { shortcutFor } from '../../lib/commands/registry';
  import { ZOOMS, addDays, axis, buildRows, dayDiff, dragDates, localDay, range, saturdays, shiftLocalDays, unscheduled, zoomBy, type Row, type Zoom } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import { priorityGlyph } from '../../lib/priority';
  import PageHeader from '../../ui/PageHeader.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import Tabs from '../../ui/Tabs.svelte';
  import Toast from '../../ui/Toast.svelte';

  // The projects route mounts this without a view, which selects projects mode. `today` is a prop so tests are not clock-bound.
  let {
    view = null,
    // A project view lays out its own projects, not issues.
    mode = view && view.entity !== 'project' ? 'issues' : 'projects',
    today = localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone),
    embedded = view !== null,
  }: { view?: SavedView | null; mode?: 'issues' | 'projects'; today?: string; embedded?: boolean } = $props();

  const ALL: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'Gantt', columns: [], include_archived: false };
  // Must equal --row-h-compact: arrows and bar tops are computed in JS.
  const ROW = 28;

  let zoomId = $state('Week');
  const zoom = $derived(zoomId as Zoom);
  let rows = $state<Row[]>([]);
  let lane = $state<Issue[]>([]);
  let relations = $state<Relation[]>([]);
  let allIssues = $state<Issue[]>([]);
  let projectList = $state<Project[]>([]);
  let statusList = $state<Status[]>([]);
  let loaded = $state(false);
  let toast = $state<{ text: string; apply: () => Promise<void> } | null>(null);
  let error = $state('');
  let drag = $state<{ id: string; mode: 'move' | 'start' | 'end'; x0: number; dx: number } | null>(null);
  let dragIssue: number | null = null;
  let timeline: HTMLElement | undefined = $state();
  let canvas: HTMLElement | undefined = $state();

  const dw = $derived(ZOOMS[zoom]);
  const span = $derived(range(rows, today));
  const days = $derived(dayDiff(span.start, span.end) + 1);
  const xOf = (date: string) => dayDiff(span.start, date) * dw;
  const hasBars = $derived(rows.some((r) => r.start));
  const scale = $derived(axis(span.start, days, zoom));
  const weekends = $derived(zoom === 'Day' || zoom === 'Week' ? saturdays(span.start, days) : []);
  const statusOf = (i: Issue) => statusList.find((s) => s.id === i.status_id);
  const categoryOf = (i: Issue) => statusOf(i)?.category;
  let ctx = $state.raw<Ctx>(emptyCtx);
  // Up to 3 displayed fields follow the title; a view without its own list keeps the plain label.
  const extra = $derived((view?.def.columns ?? []).slice(0, 3));
  const iss = (r: Row) => (r.issueId === undefined ? undefined : allIssues.find((i) => i.id === r.issueId));
  const projectOf = (id?: number) => projectList.find((p) => p.id === id);

  let queryVersion = 0;
  async function load() {
    const mine = ++queryVersion;
    const def = view ? $state.snapshot(view.def) : null;
    const currentMode = mode;
    const own = view?.entity === 'project';
    const [issues, projects, statuses] = await Promise.all([
      own ? Promise.resolve([] as Issue[]) : api.views.query(currentMode === 'issues' && def ? def : ALL),
      own ? api.views.queryProjects(def!).then((r) => r.map((x) => x.project)) : api.projects.list(),
      api.statuses.list(),
    ]);
    const live = currentMode === 'issues' && def !== null;
    const [bars, critical, rels] = live
      ? await Promise.all([
          api.schedule.bars(def!),
          api.schedule.criticalPath(def!),
          // shortcut: one call per issue, upgrade when a view holds thousands of issues
          Promise.all(issues.map((i) => api.relations.forIssue(i.id))).then((r) => r.flat()),
        ])
      : [[], [], []];
    const shown = currentMode === 'issues' ? new Set(issues.map((i) => i.project_id)) : new Set(projects.map((p) => p.id));
    const milestones: Milestone[] = (await Promise.all(projects.filter((p) => shown.has(p.id)).map((p) => api.projects.listMilestones(p.id)))).flat();
    const nextCtx = own ? emptyCtx : await loadCtx(def ?? undefined);
    if (mine !== queryVersion) return;
    ctx = nextCtx;
    allIssues = issues;
    projectList = projects;
    // Each relation comes back from both of its issues.
    relations = [...new Map(rels.filter((r) => r.kind === 'blocks').map((r) => [`${r.from}-${r.to}`, r])).values()];
    statusList = statuses;
    rows = buildRows({ mode: currentMode, issues, bars, projects, milestones, critical, statuses });
    lane = currentMode === 'issues' ? unscheduled(issues, rows) : [];
    if (!loaded) {
      loaded = true;
      queueMicrotask(toToday);
    }
  }
  $effect(() => {
    void [mode, view];
    untrack(load);
  });

  const toToday = () => {
    if (timeline) timeline.scrollLeft = Math.max(0, xOf(today) - timeline.clientWidth / 3);
  };
  // The commands (keys, palette) fire window events; this screen does the work.
  $effect(() => {
    const on: Record<string, () => void> = {
      'timeline.zoom-in': () => (zoomId = zoomBy(zoom, 1)),
      'timeline.zoom-out': () => (zoomId = zoomBy(zoom, -1)),
      'timeline.today': toToday,
    };
    for (const [k, f] of Object.entries(on)) addEventListener(`command:${k}`, f);
    return () => Object.entries(on).forEach(([k, f]) => removeEventListener(`command:${k}`, f));
  });

  const key = (id: number) => allIssues.find((i) => i.id === id)?.key ?? `#${id}`;
  const rowOf = (issueId: number) => rows.findIndex((r) => r.issueId === issueId && r.start);

  // Keeps a timed due's time and zone, and never invents a due for an estimate-derived bar.
  function patch(m: Reschedule): IssuePatch {
    const due = allIssues.find((i) => i.id === m.issue_id)?.due;
    if (!due) return { start_date: m.start_date };
    if ('Date' in due) return { start_date: m.start_date, due: { Date: m.due_date } };
    const d = dayDiff(due.DateTime.utc.slice(0, 10), m.due_date);
    return { start_date: m.start_date, due: { DateTime: { ...due.DateTime, utc: shiftLocalDays(due.DateTime.utc, due.DateTime.tz, d) } } };
  }
  // All or nothing from the user's view: on any failure show it and reload what is really stored.
  async function guarded(fn: () => Promise<unknown>) {
    error = '';
    try {
      await fn();
    } catch (e) {
      error = (e as { message?: string })?.message ?? String(e);
    }
    await load();
  }

  function down(e: MouseEvent, r: Row, mode: 'move' | 'start' | 'end' = 'move') {
    if (r.kind === 'group' || r.kind === 'milestone' || r.kind === 'rollup') return;
    e.stopPropagation();
    drag = { id: r.id, mode, x0: e.clientX, dx: 0 };
  }
  function move(e: MouseEvent) {
    if (drag) drag.dx = e.clientX - drag.x0;
  }
  const up = () => (drag ? guarded(commit) : undefined);
  async function commit() {
    if (!drag) return;
    const r = rows.find((x) => x.id === drag!.id)!;
    const d = Math.round(drag.dx / dw);
    const mode = drag.mode;
    drag = null;
    if (!d) return;
    const to = dragDates({ start: r.start!, end: r.end! }, mode, d);
    if (to.start === r.start && to.end === r.end) return;
    if (r.projectId !== undefined) {
      const p = projectList.find((x) => x.id === r.projectId)!;
      await api.projects.update({ ...p, start_date: to.start, target_date: to.end });
      return;
    }
    const moved: Reschedule = { issue_id: r.issueId!, start_date: to.start, due_date: to.end };
    const also = await api.schedule.shiftDependents(moved);
    const apply = () => {
      toast = null;
      return guarded(async () => {
        for (const m of [moved, ...also]) await api.issues.update(m.issue_id, patch(m));
      });
    };
    if (!also.length) return void (await apply());
    toast = { text: `Also moves ${also.map((a) => key(a.issue_id)).join(', ')}`, apply };
  }

  async function drop(e: DragEvent) {
    e.preventDefault();
    if (dragIssue === null) return;
    const x = e.clientX - (canvas?.getBoundingClientRect().left ?? 0);
    const id = dragIssue;
    dragIssue = null;
    await guarded(() => api.issues.update(id, { start_date: addDays(span.start, Math.floor(x / dw)) }));
  }

  // The bar as drawn: while dragging it follows the pointer in whole days.
  const live = (r: Row) => (drag?.id === r.id ? dragDates({ start: r.start!, end: r.end! }, drag.mode, Math.round(drag.dx / dw)) : { start: r.start!, end: r.end! });
  const barStyle = (r: Row) => {
    const s = live(r);
    return `left:${xOf(s.start)}px;width:${Math.max((dayDiff(s.start, s.end) + 1) * dw, 4)}px`;
  };
  // The issue whose bar the pointer is on: its connectors are drawn strong, the rest fade.
  let hot = $state<number | null>(null);
  // Blocker's end, a short step right, to the target's row, into its start. A target that starts before that step is
  // reached by dropping to the line between rows first.
  const STEP = 8;
  const arrows = $derived(
    relations.flatMap((rel) => {
      const a = rowOf(rel.from);
      const b = rowOf(rel.to);
      if (a < 0 || b < 0) return [];
      const x1 = xOf(rows[a].end!) + dw;
      const x2 = xOf(rows[b].start!);
      const y1 = a * ROW + ROW / 2;
      const y2 = b * ROW + ROW / 2;
      const ym = y2 + (b > a ? -ROW / 2 : ROW / 2);
      const d = x2 >= x1 + 2 * STEP ? `M${x1},${y1} H${x1 + STEP} V${y2} H${x2}` : `M${x1},${y1} H${x1 + STEP} V${ym} H${x2 - STEP} V${y2} H${x2}`;
      return [{ id: `${rel.from}-${rel.to}`, from: rel.from, to: rel.to, critical: !!(rows[a].critical && rows[b].critical), d }];
    }),
  );
  const tint = (r: Row) => (r.color ? `var(--c-${r.color}, var(--accent))` : 'var(--accent)');
  // A title goes inside its bar when it fits (about 6.5px a character plus padding), else just right of it.
  const inside = (r: Row) => {
    const s = live(r);
    return (dayDiff(s.start, s.end) + 1) * dw > (r.title ?? r.label).length * 6.5 + 20;
  };
</script>

<svelte:window onpointermove={move} onpointerup={up} />

{#snippet controls()}
  <Button icon="target" tooltip={{ label: 'Go to today', shortcut: shortcutFor('timeline.today') }} onclick={toToday}>Today</Button>
  <Tabs bind:value={zoomId} label="Zoom" tabs={Object.keys(ZOOMS).map((z) => ({ id: z, label: z }))} />
{/snippet}

<section class="gantt" data-timeline data-zoom={zoom}>
  {#if embedded}
    <div class="bar-controls">{@render controls()}</div>
  {:else}
    <PageHeader title="Project timeline" icon="icon:timeline" color="gray">
      {#snippet actions()}{@render controls()}{/snippet}
    </PageHeader>
  {/if}

  <div class="chart">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="timeline" bind:this={timeline} data-testid="timeline" ondragover={(e) => e.preventDefault()} ondrop={drop}>
      <div class="sheet" style="--plane-w:{days * dw}px">
        <div class="axisrow">
          <div class="corner">{mode === 'projects' ? 'Projects' : 'Issues'}</div>
          <div class="axis">
            <div class="bands">{#each scale.bands as b (b.k)}<span class="band-label" style="left:{b.k * dw}px;width:{b.len * dw}px"><span>{b.label}</span></span>{/each}</div>
            <div class="ticks">{#each scale.ticks as t (t.k)}<span class="tick" class:weekend={t.weekend} class:now={t.k === dayDiff(span.start, today) && zoom === 'Day'} style="left:{t.k * dw}px">{#if t.sub}<small>{t.sub}</small>{/if}{t.label}</span>{/each}</div>
            <div class="flag"><span class="today-chip" style="left:max({xOf(today)}px, var(--s-9))">Today</span></div>
          </div>
        </div>

        <div class="lines" role="list" aria-label="Issues">
          <!-- One element per line holds its label and its bar, so they cannot drift apart. -->
          <div class="canvas" bind:this={canvas} style="min-width:{days * dw}px" aria-hidden="true">
            {#each rows as r, n (r.id)}{#if n % 2}<div class="stripe" style="top:{n * ROW}px"></div>{/if}{/each}
            {#each weekends as k (k)}<div class="weekend" style="left:{k * dw}px;width:{2 * dw}px"></div>{/each}
            <div class="today" data-testid="today" style="left:{xOf(today)}px"></div>
            <svg class="arrows" width={days * dw} height={ROW * rows.length}>
              <defs>
                <marker id="cx-arrow" viewBox="0 0 6 6" refX="5" refY="3" markerWidth="6" markerHeight="6" orient="auto"><path d="M0,0 L6,3 L0,6 z" fill="currentColor" /></marker>
              </defs>
              {#each arrows as a (a.id)}<path class:critical={a.critical} class:hot={hot !== null && (a.from === hot || a.to === hot)} class:dim={hot !== null && a.from !== hot && a.to !== hot} d={a.d} marker-end="url(#cx-arrow)" />{/each}
            </svg>
          </div>

          {#if !loaded}
            {#each [0, 1, 2, 3, 4, 5] as n (n)}<div class="line"><div class="label"><Skeleton width="{70 - n * 6}%" height="var(--s-5)" /></div></div>{/each}
          {/if}
          {#each rows as r, n (r.id)}
            {@const p = projectOf(r.projectId)}
            <div class="line" class:alt={n % 2 === 1} data-kind={r.kind} role="listitem" data-issue={r.issueId} onpointerenter={() => (hot = r.issueId ?? null)} onpointerleave={() => (hot = null)}>
              <div class="label" data-kind={r.kind} data-label={r.id} data-slug={view?.entity === 'project' ? p?.slug : undefined} onclick={() => view?.entity === 'project' && p && navigate(`/project/${p.slug}`)} style="padding-left:calc(var(--s-6) + {r.indent} * var(--s-7))">
                {#if r.kind === 'group' && p}<EntityIcon icon={p.icon} color={p.color} size={14} />
                {:else if r.category}
                  {#if iss(r)}<span class="prio"><PriorityGlyph level={priorityGlyph(iss(r)!.priority)} size={14} /></span>{/if}
                  <StatusGlyph category={r.category} icon={r.icon} color={r.color} size={14} />
                {:else if r.kind === 'milestone'}<span class="stone" aria-hidden="true"></span>{/if}
                {#if r.key}<span class="k">{r.key}</span>{/if}
                <span class="text">{r.title ?? r.label}</span>
                {#if iss(r)}
                  {#each extra as f (f.Property ?? f.Fixed)}<span class="fv"><FieldValue field={f} issue={iss(r)!} {ctx} /></span>{/each}
                {/if}
              </div>
              <div class="cell">
                {#if r.start}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="bar {r.kind}"
                    class:critical={r.critical}
                    class:estimated={r.estimated}
                    class:dragging={drag?.id === r.id}
                    data-row={r.id}
                    data-category={r.category}
                    style="{r.kind === 'milestone' ? `left:${xOf(r.start)}px` : barStyle(r)};--bc:{tint(r)}"
                    onpointerdown={(e) => down(e, r)}
                  >
                    {#if r.kind === 'bar'}
                      <span class="handle start" data-handle="start" onpointerdown={(e) => down(e, r, 'start')}></span>
                      {#if !r.estimated}<span class="handle end" data-handle="end" onpointerdown={(e) => down(e, r, 'end')}></span>{/if}
                    {/if}
                    {#if r.kind === 'bar' && (dw >= ZOOMS.Week || inside(r))}<span class="name" class:out={!inside(r)}>{r.title ?? r.label}</span>{/if}
                  </div>
                {/if}
              </div>
            </div>
          {/each}

          {#if lane.length}
            <div class="line band unscheduled" role="listitem">
              <div class="label" data-kind="group"><span class="text">Unscheduled</span><span class="count">{lane.length}</span></div>
              <div class="cell"></div>
            </div>
            {#each lane as i (i.id)}
              <div class="line todo" role="listitem" draggable="true" data-issue={i.id} ondragstart={() => (dragIssue = i.id)}>
                <div class="label" data-label="un-{i.id}">
                  {#if categoryOf(i)}<StatusGlyph category={categoryOf(i)!} icon={statusOf(i)?.icon} color={statusOf(i)?.color} size={14} />{/if}
                  <span class="k">{i.key}</span><span class="text">{i.title}</span>
                </div>
                <div class="cell"><span class="slot">drag to schedule</span></div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
    {#if loaded && !hasBars}
      <div class="empty-layer">{#if mode === 'projects'}<EmptyState title="Nothing scheduled" hint="Give projects a start date and a target date." />{:else}<EmptyState title="Nothing scheduled" hint="Give issues a start date and a due date, or drag one from the Unscheduled band onto a day." />{/if}</div>
    {/if}
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if toast}
    <div class="toast-slot">
      <Toast>
        {toast.text}
        {#snippet action()}
          <Button variant="primary" onclick={toast!.apply}>Apply</Button>
          <Button onclick={() => (toast = null)}>Cancel</Button>
        {/snippet}
      </Toast>
    </div>
  {/if}
</section>

<style>
  .gantt { --left: 320px; --band-row: calc(var(--s-7) + var(--s-1)); --flag-row: var(--s-7); --axis-h: calc(var(--band-row) + var(--row-h) + var(--flag-row)); --grid: color-mix(in srgb, var(--ink) 7%, transparent); --tint: color-mix(in srgb, var(--ink) 3.5%, transparent); display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--sheet); color: var(--ink); }
  .bar-controls { flex: none; height: var(--topbar-h); display: flex; align-items: center; justify-content: flex-end; gap: var(--s-4); padding: 0 var(--s-7); border-bottom: 1px solid var(--border); }
  .chart { flex: 1; min-width: 0; min-height: 0; position: relative; display: flex; }
  .timeline { flex: 1; overflow: auto; }
  .sheet { width: max-content; min-width: 100%; }

  .axisrow { position: sticky; top: 0; z-index: calc(var(--z-sticky) + 2); display: grid; grid-template-columns: var(--left) minmax(var(--plane-w), 1fr); height: var(--axis-h); }
  .corner, .axis { height: var(--axis-h); background: var(--frame); color: var(--frame-muted); border-bottom: 1px solid var(--frame-inset); }
  .corner { position: sticky; left: 0; z-index: 1; display: flex; align-items: flex-end; padding: 0 var(--s-7) var(--s-4); font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; }
  .axis { position: relative; overflow: hidden; display: grid; grid-template-rows: var(--band-row) 1fr var(--flag-row); }
  .bands, .ticks, .flag { position: relative; }
  .bands { border-bottom: 1px solid var(--frame-raised); }
  .band-label { position: absolute; top: 0; bottom: 0; border-left: 1px solid var(--frame-raised); font-size: var(--fs-xs); font-weight: 700; color: var(--frame-ink); white-space: nowrap; overflow: hidden; }
  /* The label stays at the left edge of the visible chart while its month is on screen. */
  .band-label span { position: sticky; left: var(--left); display: inline-block; padding: var(--s-1) var(--s-4) 0; }
  .tick { position: absolute; top: 0; bottom: 0; display: flex; flex-direction: column; justify-content: center; padding-left: var(--s-4); border-left: 1px solid var(--frame-raised); font-size: var(--fs-xs); font-weight: 600; line-height: 1.25; white-space: nowrap; font-variant-numeric: tabular-nums; }
  .tick small { font-size: var(--fs-xs); font-weight: 500; opacity: 0.75; }
  .tick.weekend { opacity: 0.6; }
  .tick.now { color: var(--frame-ink); opacity: 1; box-shadow: inset 0 calc(-1 * var(--s-1)) 0 var(--signal); }
  .tick.now small { opacity: 1; }
  .today-chip { position: absolute; top: 0; transform: translateX(-50%); padding: 0 var(--s-3); background: var(--signal); color: var(--signal-ink); font-size: var(--fs-xs); font-weight: 700; line-height: var(--flag-row); border-radius: var(--r-sm) var(--r-sm) 0 0; }

  .lines { position: relative; }
  .line { position: relative; display: grid; grid-template-columns: var(--left) minmax(var(--plane-w), 1fr); height: var(--row-h-compact); border-bottom: 1px solid var(--grid); }
  .line { --row-bg: var(--sheet); }
  .line.alt { --row-bg: color-mix(in srgb, var(--ink) 3.5%, var(--sheet)); }
  .line:hover { --row-bg: color-mix(in srgb, var(--ink) 7%, var(--sheet)); background: var(--tint); }
  .line.alt .label, .line:hover .label { background: var(--row-bg); }
  .line[data-kind='group'] { background: var(--band-1); }
  .line.band { background: var(--band-2); }
  .line.todo { cursor: grab; }
  .line.todo:hover { background: var(--active); }
  .label { position: sticky; left: 0; z-index: calc(var(--z-sticky) + 1); display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-5) 0 var(--s-6); white-space: nowrap; overflow: hidden; background: var(--sheet); border-right: 1px solid var(--border); }
  .line[data-kind='group'] .label, .line.band .label { background: inherit; color: var(--band-ink); font-weight: 700; }
  .line.band .label { text-transform: uppercase; font-size: var(--fs-sm); letter-spacing: 0.04em; }
  .line.todo:hover .label { background: var(--active); }
  .label .text { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .label .fv { flex: none; max-width: 96px; display: inline-flex; color: var(--muted); font-size: var(--fs-sm); }
  .label .prio { flex: none; display: inline-flex; }
  .label .k { flex: none; color: var(--muted); font-weight: 600; font-size: var(--fs-sm); }
  .label[data-kind='milestone'] { color: var(--muted); }
  .label[data-kind='rollup'] { font-weight: 600; }
  .count { color: var(--band-count); }
  .stone { width: var(--s-5); height: var(--s-5); background: var(--stone); transform: rotate(45deg); flex: none; }
  .cell { position: relative; }
  .slot { position: absolute; left: var(--s-6); top: var(--s-2); bottom: var(--s-2); display: flex; align-items: center; padding: 0 var(--s-5); border: 1.5px dashed var(--control); border-radius: var(--r-sm); color: var(--muted); font-size: var(--fs-xs); white-space: nowrap; }

  .canvas { position: absolute; top: 0; bottom: 0; left: var(--left); right: 0; pointer-events: none; }
  .stripe { position: absolute; left: 0; right: 0; height: var(--row-h-compact); background: var(--tint); }
  .weekend { position: absolute; top: 0; bottom: 0; background: var(--tint); border-left: 1px solid var(--grid); }
  .today { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--signal); opacity: 0.7; z-index: 1; }
  .arrows { position: absolute; top: 0; left: 0; color: var(--muted); z-index: 1; }
  .arrows path { fill: none; stroke: currentColor; stroke-width: 1; stroke-linejoin: round; opacity: 0.7; }
  .arrows path.critical { color: var(--signal); }
  .arrows path.dim { opacity: 0.3; }
  .arrows path.hot, .arrows path.hot.critical { color: var(--accent); stroke-width: 2; opacity: 1; }
  .arrows :global(marker path) { fill: currentColor; stroke: none; }

  /* A bar is its status colour (--bc) as a tint with a stronger edge; the title sits inside or just right of it. */
  .bar { --bc: var(--accent); position: absolute; top: var(--s-2); z-index: 2; height: calc(var(--row-h-compact) - 2 * var(--s-2) - 1px); box-sizing: border-box; border-radius: var(--r-sm); background: color-mix(in srgb, var(--bc) 30%, var(--sheet)); border: 1px solid color-mix(in srgb, var(--bc) 75%, transparent); border-left-width: var(--s-2); color: var(--ink); cursor: grab; display: flex; align-items: center; transition: filter var(--dur-1) var(--ease); }
  .bar:hover { filter: brightness(1.15); }
  .bar.dragging { cursor: grabbing; outline: 2px solid var(--focus); }
  .bar .name { padding: 0 var(--s-4); font-size: var(--fs-xs); font-weight: 500; white-space: nowrap; overflow: hidden; pointer-events: none; }
  .bar .name.out { position: absolute; left: 100%; margin-left: var(--s-7); padding: 0; overflow: visible; color: var(--muted); text-shadow: 0 0 var(--s-1) var(--row-bg), 0 0 var(--s-1) var(--row-bg), 0 0 var(--s-2) var(--row-bg), 0 0 var(--s-2) var(--row-bg), 0 0 var(--s-2) var(--row-bg), 0 0 var(--s-2) var(--row-bg), 0 0 var(--s-3) var(--row-bg), 0 0 var(--s-3) var(--row-bg); }
  .bar[data-category='Completed'], .bar[data-category='Canceled'] { opacity: 0.55; }
  .bar[data-category='Canceled'] .name { text-decoration: line-through; }
  /* Estimated (no due date): a real bar, its last third fading out and ending in a dotted edge. */
  .bar.estimated { background: linear-gradient(90deg, transparent 66%, color-mix(in srgb, var(--sheet) 60%, transparent)), color-mix(in srgb, var(--bc) 30%, var(--sheet)); border-right-style: dotted; }
  /* On the critical path: a signal edge all round, fill unchanged. */
  .bar.critical { border-color: var(--signal); border-left-color: var(--signal); box-shadow: 0 0 0 1px color-mix(in srgb, var(--signal) 45%, transparent); }
  .bar.rollup { height: var(--s-4); top: var(--s-6); background: var(--ink); border: 0; cursor: default; border-radius: 0; }
  .bar.group { height: var(--s-2); top: calc(var(--row-h-compact) - var(--s-4)); background: var(--band-count); border: 0; cursor: default; border-radius: 0; }
  .bar.milestone { width: var(--s-6); height: var(--s-6); top: var(--s-4); margin-left: calc(var(--s-6) / -2); background: var(--stone); border: 0; transform: rotate(45deg); cursor: default; border-radius: var(--r-sm); }
  .handle { position: absolute; top: 0; bottom: 0; width: var(--s-3); cursor: ew-resize; }
  .handle.start { left: 0; }
  .handle.end { right: 0; }
  .handle:hover { background: var(--overlay); }

  .empty-layer { position: absolute; inset: var(--axis-h) 0 0 var(--left); display: grid; place-items: center; pointer-events: none; }
  .error { margin: 0; padding: var(--s-4) var(--s-7); background: var(--signal); color: var(--signal-ink); font-weight: 600; }
  .toast-slot { position: fixed; bottom: var(--s-8); left: 50%; transform: translateX(-50%); z-index: var(--z-toast); }
</style>

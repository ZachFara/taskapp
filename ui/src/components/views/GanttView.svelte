<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../../lib/api';
  import type { Issue, IssuePatch, Milestone, Project, Relation, Reschedule, SavedView, ViewDef } from '../../lib/bindings';
  import { shortcutFor } from '../../lib/commands/registry';
  import { TICK_DAYS, ZOOMS, addDays, buildRows, dayDiff, dragDates, localDay, range, saturdays, shiftLocalDays, tickLabel, unscheduled, zoomBy, type Row, type Zoom } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import Tabs from '../../ui/Tabs.svelte';
  import Toast from '../../ui/Toast.svelte';

  // The projects route mounts this without a view, which selects projects mode. `today` is a prop so tests are not clock-bound.
  let {
    view = null,
    mode = view ? 'issues' : 'projects',
    today = localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone),
  }: { view?: SavedView | null; mode?: 'issues' | 'projects'; today?: string } = $props();

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
  const ticks = $derived(Array.from({ length: Math.ceil(days / TICK_DAYS[zoom]) }, (_, k) => addDays(span.start, k * TICK_DAYS[zoom])));
  const weekends = $derived(zoom === 'Day' || zoom === 'Week' ? saturdays(span.start, days) : []);
  const projectOf = (id?: number) => projectList.find((p) => p.id === id);

  async function load() {
    const id = view?.id ?? null;
    const [issues, projects, statuses] = await Promise.all([api.views.query(mode === 'issues' && view ? view.def : ALL), api.projects.list(), api.statuses.list()]);
    const live = mode === 'issues' && id !== null;
    const [bars, critical, rels] = live
      ? await Promise.all([
          api.schedule.bars(id!),
          api.schedule.criticalPath(id!),
          // shortcut: one call per issue, upgrade when a view holds thousands of issues
          Promise.all(issues.map((i) => api.relations.forIssue(i.id))).then((r) => r.flat()),
        ])
      : [[], [], []];
    const shown = mode === 'issues' ? new Set(issues.map((i) => i.project_id)) : new Set(projects.map((p) => p.id));
    const milestones: Milestone[] = (await Promise.all(projects.filter((p) => shown.has(p.id)).map((p) => api.projects.listMilestones(p.id)))).flat();
    allIssues = issues;
    projectList = projects;
    // Each relation comes back from both of its issues.
    relations = [...new Map(rels.filter((r) => r.kind === 'Blocks').map((r) => [`${r.from}-${r.to}`, r])).values()];
    lane = unscheduled(issues);
    rows = buildRows({ mode, issues, bars, projects, milestones, critical, statuses });
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
  // Blocker's end to the blocked bar's start.
  const arrows = $derived(
    relations.flatMap((rel) => {
      const a = rowOf(rel.from);
      const b = rowOf(rel.to);
      if (a < 0 || b < 0) return [];
      const x1 = xOf(rows[a].end!) + dw;
      const x2 = xOf(rows[b].start!);
      const y1 = a * ROW + ROW / 2;
      const y2 = b * ROW + ROW / 2;
      return [{ id: `${rel.from}-${rel.to}`, critical: !!(rows[a].critical && rows[b].critical), d: `M${x1},${y1} H${x1 + 6} V${y2} H${Math.max(x2, x1 + 6)}` }];
    }),
  );
</script>

<svelte:window onpointermove={move} onpointerup={up} />

<section class="gantt" data-timeline data-zoom={zoom}>
  <PageHeader title={mode === 'projects' ? 'Project timeline' : 'Timeline'} icon="icon:timeline" color="gray">
    {#snippet actions()}
      <Button icon="target" tooltip={{ label: 'Go to today', shortcut: shortcutFor('timeline.today') }} onclick={toToday}>Today</Button>
      <Tabs bind:value={zoomId} label="Zoom" tabs={Object.keys(ZOOMS).map((z) => ({ id: z, label: z }))} />
    {/snippet}
  </PageHeader>

  <div class="body">
    <div class="chart">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="timeline" bind:this={timeline} data-testid="timeline" ondragover={(e) => e.preventDefault()} ondrop={drop}>
        <div class="sheet" style="grid-template-columns:var(--sidebar-w) {days * dw}px">
          <div class="corner"></div>
          <div class="axis" style="width:{days * dw}px">
            {#each ticks as t (t)}<span class="tick" style="left:{xOf(t)}px">{tickLabel(t, zoom)}</span>{/each}
            <span class="today-chip" style="left:{xOf(today)}px">Today</span>
          </div>

          <div class="labels" role="list" aria-label="Issues">
            {#if !loaded}
              {#each [0, 1, 2, 3, 4, 5] as n (n)}<div class="label"><Skeleton width="{70 - n * 6}%" height="var(--s-5)" /></div>{/each}
            {/if}
            {#each rows as r (r.id)}
              {@const p = projectOf(r.projectId)}
              <div class="label {r.kind}" class:group={r.kind === 'group'} role="listitem" data-issue={r.issueId} style="padding-left:calc(var(--s-6) + {r.indent} * var(--s-7))">
                {#if r.kind === 'group' && p}<EntityIcon icon={p.icon} color={p.color} size={14} />
                {:else if r.category}<StatusGlyph category={r.category} size={14} />
                {:else if r.kind === 'milestone'}<span class="stone" aria-hidden="true"></span>{/if}
                <span class="text">{r.label}</span>
              </div>
            {/each}
          </div>

          <div class="canvas" bind:this={canvas} style="width:{days * dw}px;height:{ROW * rows.length}px">
            {#each weekends as k (k)}<div class="weekend" style="left:{k * dw}px;width:{2 * dw}px"></div>{/each}
            {#each rows as r, n (r.id)}
              {#if r.kind === 'group'}<div class="gband" style="top:{n * ROW}px"></div>{/if}
            {/each}
            <div class="today" data-testid="today" style="left:{xOf(today)}px"></div>
            <svg class="arrows" width={days * dw} height={ROW * rows.length} aria-hidden="true">
              <defs>
                <marker id="cx-arrow" viewBox="0 0 6 6" refX="5" refY="3" markerWidth="6" markerHeight="6" orient="auto"><path d="M0,0 L6,3 L0,6 z" fill="currentColor" /></marker>
              </defs>
              {#each arrows as a (a.id)}<path class:critical={a.critical} d={a.d} marker-end="url(#cx-arrow)" />{/each}
            </svg>
            {#each rows as r, n (r.id)}
              {#if r.start}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="bar {r.kind}"
                  class:critical={r.critical}
                  class:estimated={r.estimated}
                  class:dragging={drag?.id === r.id}
                  data-row={r.id}
                  data-category={r.category}
                  style="top:{n * ROW}px;{r.kind === 'milestone' ? `left:${xOf(r.start)}px` : barStyle(r)}"
                  onpointerdown={(e) => down(e, r)}
                >
                  {#if r.kind === 'bar'}
                    <span class="handle start" data-handle="start" onpointerdown={(e) => down(e, r, 'start')}></span>
                    {#if !r.estimated}<span class="handle end" data-handle="end" onpointerdown={(e) => down(e, r, 'end')}></span>{/if}
                  {/if}
                </div>
              {/if}
            {/each}
          </div>
        </div>
      </div>
      {#if loaded && !hasBars}
        <div class="empty-layer"><EmptyState title="Nothing scheduled" hint="Give issues a start date and a due date, or drag one from the Unscheduled list onto a day." /></div>
      {/if}
    </div>

    <aside class="lane" aria-label="Unscheduled">
      <h3>Unscheduled <span class="count">{lane.length}</span></h3>
      <div role="list">
        {#each lane as i (i.id)}
          <div class="item" draggable="true" data-issue={i.id} ondragstart={() => (dragIssue = i.id)} role="listitem">
            <span class="k">{i.key}</span> <span class="t">{i.title}</span>
          </div>
        {:else}
          <p class="muted">Nothing to schedule.</p>
        {/each}
      </div>
    </aside>
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
  .gantt { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--sheet); color: var(--ink); }
  .body { display: flex; flex: 1; min-height: 0; }
  .chart { flex: 1; min-width: 0; position: relative; display: flex; }
  .timeline { flex: 1; overflow: auto; }
  .sheet { display: grid; grid-template-rows: var(--row-h) auto; }

  .corner, .axis { position: sticky; top: 0; z-index: var(--z-sticky); height: var(--row-h); background: var(--frame); color: var(--frame-muted); border-bottom: 1px solid var(--frame-inset); }
  .corner { left: 0; z-index: calc(var(--z-sticky) + 2); }
  .axis { overflow: hidden; }
  .tick { position: absolute; top: 0; height: 100%; display: flex; align-items: center; padding-left: var(--s-3); border-left: 1px solid var(--frame-raised); font-size: var(--fs-xs); font-weight: 500; white-space: nowrap; }
  .today-chip { position: absolute; bottom: 0; transform: translateX(-50%); padding: 0 var(--s-3); background: var(--signal); color: var(--signal-ink); font-size: var(--fs-xs); font-weight: 700; border-radius: var(--r-sm) var(--r-sm) 0 0; }

  .labels { position: sticky; left: 0; z-index: calc(var(--z-sticky) + 1); background: var(--sheet); border-right: 1px solid var(--border); }
  .label { height: var(--row-h-compact); display: flex; align-items: center; gap: var(--s-4); padding-right: var(--s-5); white-space: nowrap; overflow: hidden; border-bottom: 1px solid var(--border); }
  .label .text { overflow: hidden; text-overflow: ellipsis; }
  .label.group { background: var(--band-1); color: var(--band-ink); font-weight: 700; letter-spacing: -0.005em; }
  .label.milestone { color: var(--muted); }
  .label.rollup { font-weight: 600; }
  .stone { width: var(--s-5); height: var(--s-5); background: var(--stone); transform: rotate(45deg); flex: none; }

  .canvas { position: relative; background-image: linear-gradient(var(--border) 1px, transparent 1px); background-size: 100% var(--row-h-compact); background-position: 0 calc(var(--row-h-compact) - 1px); }
  .weekend { position: absolute; top: 0; bottom: 0; background: var(--raised); }
  .gband { position: absolute; left: 0; right: 0; height: var(--row-h-compact); background: var(--band-1); }
  .today { position: absolute; top: 0; bottom: 0; width: var(--s-1); margin-left: calc(var(--s-1) / -2); background: var(--signal); z-index: 2; pointer-events: none; }
  .arrows { position: absolute; top: 0; left: 0; pointer-events: none; color: var(--muted); z-index: 3; }
  .arrows path { fill: none; stroke: currentColor; stroke-width: 1.5; }
  .arrows path.critical { color: var(--signal); }
  .arrows :global(marker path) { fill: currentColor; stroke: none; }

  .bar { position: absolute; z-index: 4; height: calc(var(--row-h-compact) - 2 * var(--s-2)); margin-top: var(--s-2); border-radius: var(--r-sm); background: var(--control); color: var(--ink); cursor: grab; transition: filter var(--dur-1) var(--ease); }
  .bar:hover { filter: brightness(0.94); }
  .bar.dragging { cursor: grabbing; outline: 2px solid var(--focus); }
  .bar[data-category='Backlog'] { background: var(--sheet); outline: 1.5px dashed var(--control); outline-offset: -1.5px; }
  .bar[data-category='Unstarted'] { background: var(--control); }
  .bar[data-category='Started'] { background: var(--accent); }
  .bar[data-category='Completed'] { background: var(--stone); }
  .bar[data-category='Canceled'] { background: var(--border); opacity: 0.7; }
  .bar.estimated { opacity: 0.6; background-image: repeating-linear-gradient(135deg, transparent 0 var(--s-3), var(--sheet) var(--s-3) var(--s-4)); }
  .bar.critical { background: var(--signal); }
  .bar.rollup { height: var(--s-4); margin-top: var(--s-6); background: var(--ink); cursor: default; border-radius: 0; }
  .bar.group { height: var(--s-2); margin-top: calc(var(--row-h-compact) - var(--s-3)); background: var(--band-count); cursor: default; border-radius: 0; }
  .bar.milestone { width: var(--s-6); height: var(--s-6); margin-top: var(--s-4); margin-left: calc(var(--s-6) / -2); background: var(--stone); transform: rotate(45deg); cursor: default; border-radius: var(--r-sm); }
  .handle { position: absolute; top: 0; bottom: 0; width: var(--s-3); cursor: ew-resize; }
  .handle.start { left: 0; }
  .handle.end { right: 0; }
  .handle:hover { background: var(--overlay); }

  .empty-layer { position: absolute; inset: var(--row-h) 0 0 var(--sidebar-w); display: grid; place-items: center; pointer-events: none; }

  .lane { width: var(--sidebar-w); flex: none; border-left: 1px solid var(--border); background: var(--raised); overflow: auto; }
  .lane h3 { height: var(--row-h); display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-6); background: var(--band-2); color: var(--band-ink); font-size: var(--fs-sm); font-weight: 700; letter-spacing: 0.02em; text-transform: uppercase; }
  .count { color: var(--band-count); }
  .item { padding: var(--s-4) var(--s-6); border-bottom: 1px solid var(--border); cursor: grab; display: flex; gap: var(--s-4); overflow: hidden; white-space: nowrap; }
  .item:hover { background: var(--active); }
  .item .k { color: var(--muted); font-weight: 600; }
  .item .t { overflow: hidden; text-overflow: ellipsis; }
  .muted { color: var(--muted); padding: var(--s-6); }
  .error { margin: 0; padding: var(--s-4) var(--s-7); background: var(--signal); color: var(--signal-ink); font-weight: 600; }
  .toast-slot { position: fixed; bottom: var(--s-8); left: 50%; transform: translateX(-50%); z-index: var(--z-toast); }
</style>

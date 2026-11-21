<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../../lib/api';
  import type { Issue, IssuePatch, Milestone, Project, Relation, Reschedule, SavedView, ViewDef } from '../../lib/bindings';
  import { TICK_DAYS, ZOOMS, addDays, buildRows, dayDiff, range, unscheduled, type Row, type Zoom } from '../../lib/gantt';

  // The projects route mounts this without a view, which selects projects mode.
  let { view = null, mode = view ? 'issues' : 'projects' }: { view?: SavedView | null; mode?: 'issues' | 'projects' } = $props();

  const ALL: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'Gantt', columns: [], include_archived: false };
  const ROW = 28;
  const today = new Date().toISOString().slice(0, 10);

  let zoom = $state<Zoom>('Week');
  let rows = $state<Row[]>([]);
  let lane = $state<Issue[]>([]);
  let relations = $state<Relation[]>([]);
  let allIssues = $state<Issue[]>([]);
  let projectList = $state<Project[]>([]);
  let toast = $state<{ text: string; apply: () => Promise<void> } | null>(null);
  let error = $state('');
  let drag = $state<{ id: string; x0: number; dx: number } | null>(null);
  let dragIssue: number | null = null;
  let timeline: HTMLElement | undefined = $state();

  const dw = $derived(ZOOMS[zoom]);
  const span = $derived(range(rows, today));
  const days = $derived(dayDiff(span.start, span.end) + 1);
  const xOf = (date: string) => dayDiff(span.start, date) * dw;
  const hasBars = $derived(rows.some((r) => r.start));
  const ticks = $derived(Array.from({ length: Math.ceil(days / TICK_DAYS[zoom]) }, (_, k) => addDays(span.start, k * TICK_DAYS[zoom])));

  async function load() {
    const id = view?.id ?? null;
    const [issues, projects] = await Promise.all([api.views.query(mode === 'issues' && view ? view.def : ALL), api.projects.list()]);
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
    rows = buildRows({ mode, issues, bars, projects, milestones, critical });
  }
  $effect(() => {
    void [mode, view];
    untrack(load);
  });

  const key = (id: number) => allIssues.find((i) => i.id === id)?.key ?? `#${id}`;
  const rowOf = (issueId: number) => rows.findIndex((r) => r.issueId === issueId && r.start);
  const shifted = (r: Row, d: number) => ({ start: addDays(r.start!, d), end: addDays(r.end!, d) });

  // Keeps a timed due's time and zone, and never invents a due for an estimate-derived bar.
  function patch(m: Reschedule): IssuePatch {
    const old = allIssues.find((i) => i.id === m.issue_id);
    const due = old?.due;
    if (!due) return { start_date: m.start_date };
    if ('Date' in due) return { start_date: m.start_date, due: { Date: m.due_date } };
    const day = due.DateTime.utc.slice(0, 10);
    const d = dayDiff(old!.start_date ?? day, old!.start_date ? m.start_date : m.due_date);
    return { start_date: m.start_date, due: { DateTime: { ...due.DateTime, utc: addDays(day, d) + due.DateTime.utc.slice(10) } } };
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

  function down(e: MouseEvent, r: Row) {
    if (r.kind === 'group' || r.kind === 'milestone' || r.kind === 'rollup') return;
    drag = { id: r.id, x0: e.clientX, dx: 0 };
  }
  function move(e: MouseEvent) {
    if (drag) drag.dx = e.clientX - drag.x0;
  }
  const up = () => (drag ? guarded(commit) : undefined);
  async function commit() {
    if (!drag) return;
    const r = rows.find((x) => x.id === drag!.id)!;
    const d = Math.round(drag.dx / dw);
    drag = null;
    if (!d) return;
    const to = shifted(r, d);
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
    const x = e.clientX - (timeline?.getBoundingClientRect().left ?? 0) + (timeline?.scrollLeft ?? 0);
    const id = dragIssue;
    dragIssue = null;
    await guarded(() => api.issues.update(id, { start_date: addDays(span.start, Math.floor(x / dw)) }));
  }

  const barStyle = (r: Row) => {
    const dx = drag?.id === r.id ? Math.round(drag.dx / dw) * dw : 0;
    return `left:${xOf(r.start!) + dx}px;width:${Math.max((dayDiff(r.start!, r.end!) + 1) * dw, 4)}px`;
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
      return [{ id: `${rel.from}-${rel.to}`, d: `M${x1},${y1} H${x1 + 6} V${y2} H${Math.max(x2, x1 + 6)}` }];
    }),
  );
</script>

<svelte:window onpointermove={move} onpointerup={up} />

<section class="gantt" data-zoom={zoom}>
  <header>
    <strong>{mode === 'projects' ? 'Project timeline' : 'Gantt'}</strong>
    <div role="group" aria-label="Zoom">
      {#each Object.keys(ZOOMS) as z (z)}
        <button type="button" aria-pressed={zoom === z} onclick={() => (zoom = z as Zoom)}>{z}</button>
      {/each}
    </div>
  </header>

  <div class="body">
    <div class="chart">
      {#if !hasBars}<p class="empty">Nothing scheduled in this view yet.</p>{/if}
      <div class="labels">
        <div class="axis-pad"></div>
        {#each rows as r (r.id)}
          <div class="label" class:group={r.kind === 'group'} style="padding-left:{8 + r.indent * 14}px">{r.label}</div>
        {/each}
      </div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="timeline" bind:this={timeline} data-testid="timeline" ondragover={(e) => e.preventDefault()} ondrop={drop}>
        <div class="canvas" style="width:{days * dw}px;height:{ROW * (rows.length + 1)}px">
          <div class="axis">
            {#each ticks as t (t)}<span style="left:{xOf(t)}px">{t.slice(5)}</span>{/each}
          </div>
          <svg class="arrows" width={days * dw} height={ROW * rows.length}>
            {#each arrows as a (a.id)}<path d={a.d} />{/each}
          </svg>
          {#each rows as r, n (r.id)}
            {#if r.start}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="bar {r.kind}"
                class:critical={r.critical}
                class:estimated={r.estimated}
                data-row={r.id}
                style="top:{(n + 1) * ROW + 5}px;{r.kind === 'milestone' ? `left:${xOf(r.start)}px` : barStyle(r)}"
                onpointerdown={(e) => down(e, r)}
              ></div>
            {/if}
          {/each}
        </div>
      </div>
    </div>

    <aside class="lane" aria-label="Unscheduled">
      <h3>Unscheduled</h3>
      {#each lane as i (i.id)}
        <div class="item" draggable="true" data-issue={i.id} ondragstart={() => (dragIssue = i.id)} role="listitem">{i.key} {i.title}</div>
      {:else}
        <p class="muted">Nothing to schedule.</p>
      {/each}
    </aside>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if toast}
    <div class="toast" role="alertdialog">
      <span>{toast.text}</span>
      <button type="button" onclick={toast.apply}>Apply</button>
      <button type="button" onclick={() => (toast = null)}>Cancel</button>
    </div>
  {/if}
</section>

<style>
  .gantt { display: flex; flex-direction: column; height: 100vh; font: 13px system-ui, sans-serif; color: #222; }
  header { display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; border-bottom: 1px solid #ddd; }
  header button { border: 1px solid #ccc; background: #fff; padding: 2px 8px; }
  header button[aria-pressed='true'] { background: #222; color: #fff; }
  .body { display: flex; flex: 1; min-height: 0; }
  .chart { display: flex; flex: 1; min-width: 0; position: relative; }
  .empty { position: absolute; top: 48px; left: 220px; color: #888; }
  .labels { width: 200px; flex: none; border-right: 1px solid #ddd; }
  .axis-pad { height: 28px; }
  .label { height: 28px; line-height: 28px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .label.group { font-weight: 600; }
  .timeline { flex: 1; overflow: auto; }
  .canvas { position: relative; }
  .axis { position: relative; height: 28px; border-bottom: 1px solid #eee; }
  .axis span { position: absolute; top: 6px; font-size: 11px; color: #888; }
  .arrows { position: absolute; top: 28px; left: 0; pointer-events: none; }
  .arrows path { fill: none; stroke: #888; stroke-width: 1; }
  .bar { position: absolute; height: 18px; border-radius: 4px; background: #6b7cff; cursor: grab; }
  .bar.rollup { background: #444; height: 8px; margin-top: 5px; cursor: default; }
  .bar.estimated { opacity: 0.6; }
  .bar.critical { background: #d9534f; }
  .bar.milestone { width: 12px; height: 12px; margin-top: 3px; background: #e5a50a; transform: rotate(45deg); cursor: default; }
  .lane { width: 220px; flex: none; border-left: 1px solid #ddd; padding: 8px; overflow: auto; }
  .lane h3 { margin: 0 0 8px; font-size: 12px; text-transform: uppercase; color: #666; }
  .item { padding: 4px 6px; border: 1px solid #e2e2e2; border-radius: 4px; margin-bottom: 4px; cursor: grab; }
  .error { margin: 0; padding: 6px 12px; color: #b00020; }
  .muted { color: #888; }
  .toast { position: fixed; bottom: 16px; left: 50%; transform: translateX(-50%); background: #222; color: #fff; padding: 8px 12px; border-radius: 6px; display: flex; gap: 8px; align-items: center; }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Cycle, Issue, Placement, SavedView, Status, ViewFolder } from '../lib/bindings';
  import { buildRows } from '../lib/folderTree';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import GroupHeader from '../ui/GroupHeader.svelte';
  import Icon from '../ui/Icon.svelte';
  import ListRow from '../ui/ListRow.svelte';
  import RowActions from '../ui/RowActions.svelte';
  import IconColorPicker from './pickers/IconColorPicker.svelte';
  import { rowActs, rowUi } from '../lib/rowUi.svelte';
  import { patch } from '../lib/rowActions';
    import { localDay } from '../lib/gantt';
  import { workspaceZone, dayText } from './views/dates';
  import PageHeader from '../ui/PageHeader.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import ListView from './views/ListView.svelte';
  import BarChart from './charts/BarChart.svelte';
  import CycleRetro from './CycleRetro.svelte';
  import CycleBurnup from './insights/CycleBurnup.svelte';
  import CycleSpark from './insights/CycleSpark.svelte';
  import { chartsOn, defaultMeasure, unitOf } from './insights/estimation';
  import { insightsApi } from './insights/insightsApi';

  type Band = 'current' | 'upcoming' | 'past';
  type Row = Cycle & { band: Band; done: number; total: number };

  const DAY = 86_400_000;
  const TONE: Record<Band, 1 | 2 | 3 | 4> = { current: 3, upcoming: 2, past: 4 };
  const LABEL: Record<Band, string> = { current: 'Current', upcoming: 'Upcoming', past: 'Past' };

  let rows = $state<Row[]>([]);
  // The cycles as the store holds them: what the retrospective writes back.
  let stored = $state<Cycle[]>([]);
  // Ids of pinned cycles, as the pins store them.
  let pinned = $state<Set<string>>(new Set());
  async function loadPins() {
    pinned = new Set((await api.pins.listResolved()).filter((r) => r.pin.kind === 'Cycle').map((r) => r.pin.target));
  }
  let statuses = $state<Status[]>([]);
  let folders = $state<ViewFolder[]>([]);
  let places = $state<Placement[]>([]);
  let loaded = $state(false);
  let charts = $state(false);
  let bars = $state<{ label: string; value: number }[]>([]);
  let unit = $state('issues');
  let opened = $state<number | null>(null);
  // `/cycles#<number>` opens that cycle's retrospective (the command menu and the dashboard link there).
  const fromHash = () => {
    const r = rows.find((x) => `#${x.number}` === location.hash);
    if (r) collapsed[r.band] = false;
    return r?.id ?? null;
  };
  const open = (id: number | null) => {
    opened = id;
    history.replaceState(null, '', id === null ? '/cycles' : `#${rows.find((r) => r.id === id)?.number}`);
  };
  let collapsed = $state<Record<Band, boolean>>({ current: false, upcoming: false, past: true });
  let creating = $state(false);
  let moving = $state(false);

  // The command menu and keys open these dialogs through the registry's events.
  $effect(() => {
    const on = (id: string, open: () => void) => {
      addEventListener(id, open);
      return () => removeEventListener(id, open);
    };
    const offs = [on('command:cycle.new', () => (creating = true)), on('command:cycle.move_unfinished', () => (moving = true))];
    return () => offs.forEach((off) => off());
  });
  let name = $state('');
  let start = $state('');
  let end = $state('');

  // Today in the workspace zone, as a day string; dates in the store are day strings, so they compare as text.
  let todayIn = $state(localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone));
  const bandOf = (c: Cycle, today: string): Band => (c.start_date > today ? 'upcoming' : c.end_date < today ? 'past' : 'current');
  const title = (c: Cycle) => c.name ?? `Cycle ${c.number}`;
  const cycleView = (c: Cycle): SavedView => ({
    id: 0,
    name: title(c),
    position: 0,
    icon: null,
    color: null,
    def: { filter: { Cond: { field: { Fixed: 'Cycle' }, op: 'Eq', value: c.id } }, sorts: [], group_by: { Fixed: 'Status' }, layout: 'List', columns: [], include_archived: false },
  });
  const finished = (i: Issue) => ['Completed', 'Canceled'].includes(statuses.find((s) => s.id === i.status_id)?.category ?? '');

  async function load() {
    const today = localDay(new Date(), await workspaceZone());
    todayIn = today;
    // `finished` reads the statuses, so they load before the counts are made.
    statuses = await api.statuses.list();
    const list = await api.cycles.list();
    stored = list;
    [folders, places] = await Promise.all([api.folders.list(), api.folders.items()]);
    const issues = await Promise.all(list.map((c) => api.views.query(cycleView(c).def)));
    rows = list
      .map((c, i) => ({ ...c, band: bandOf(c, today), done: issues[i].filter(finished).length, total: issues[i].length}))
      .sort((a, b) => a.start_date.localeCompare(b.start_date));
    charts = await chartsOn();
    // Each cycle's own scope, so the bar is the work that belonged to that cycle; the period that starts with it is its entry.
    const measure = await defaultMeasure();
    unit = unitOf(measure);
    bars = charts
      ? await Promise.all(rows.filter((r) => r.band !== 'upcoming').map(async (r) => ({
          label: title(r),
          value: (await insightsApi.velocity({ Cycle: r.id }, measure, rows.length)).find((p) => p.start === r.start_date)?.completed ?? 0,
        })))
      : [];
    loaded = true;
    opened ??= fromHash();
  }

  const bands = $derived((['current', 'upcoming', 'past'] as Band[]).map((b) => ({ band: b, items: rows.filter((r) => r.band === b) })));
  const current = $derived(rows.find((r) => r.band === 'current'));
  const next = $derived(rows.find((r) => r.band === 'upcoming'));
  const daysLeft = (c: Cycle) => Math.max(0, Math.ceil((Date.parse(c.end_date) - Date.parse(todayIn)) / DAY));

  async function moveUnfinished(from: Cycle, to: Cycle) {
    const issues = await api.views.query(cycleView(from).def);
    // One bulk write, so it is one undo step.
    await api.issues.bulkUpdate(issues.filter((x) => !finished(x)).map((x) => x.id), { patch: { cycle_id: to.id } });
    await load();
  }

  async function confirmMove() {
    if (current && next) await moveUnfinished(current, next);
    moving = false;
  }

  async function create() {
    if (!start || !end) return;
    await api.cycles.create(name.trim() || null, start, end, null, null);
    name = '';
    start = '';
    end = '';
    creating = false;
    await load();
  }

  onMount(() => {
    void load();
    void loadPins();
    const reload = () => (void load(), void loadPins());
    const jump = () => (opened = fromHash() ?? opened);
    addEventListener('rows:changed', reload);
    addEventListener('popstate', jump);
    return () => (removeEventListener('rows:changed', reload), removeEventListener('popstate', jump));
  });
</script>

<PageHeader title="Cycles" icon="icon:cycle">
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New cycle</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && rows.length === 0}
    <EmptyState title="No cycles yet" hint="A cycle is a dated slice of work. Start one to see what is due this week.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={() => (creating = true)}>New cycle</Button>{/snippet}
    </EmptyState>
  {:else}
    {#if charts && bars.length >= 2 && bars.some((v) => v.value > 0)}
      <section class="velocity" data-velocity><h3>Completed per cycle</h3><BarChart {bars} {unit} title="Completed per cycle" /></section>
    {/if}
    {#each bands as group (group.band)}
      {#if group.items.length > 0}
        <section data-group={group.band} class="group">
          <GroupHeader name={LABEL[group.band]} count={group.items.length} tone={TONE[group.band]} collapsed={collapsed[group.band]} ontoggle={() => (collapsed[group.band] = !collapsed[group.band])}>
            {#snippet leading()}<Icon name="cycle" size={12} />{/snippet}
          </GroupHeader>
          {#if !collapsed[group.band]}
            <ul class="rows">
              {#each buildRows('cycle', folders, places, group.items, { hideEmpty: true }) as r (r.type === 'folder' ? `f${r.folder.id}` : `c${r.item.id}`)}
               {#if r.type === 'folder'}
                <li class="fold" data-folder-id={r.folder.id} style:padding-left={`calc(${r.depth} * var(--s-8))`}><Icon name="folder" size={12} />{r.folder.name}</li>
               {:else}
                {@const c = r.item}
                <li class="row" data-cycle={c.id} style:padding-left={`calc(${r.depth} * var(--s-8))`}>
                  <RowActions label={title(c)} actions={rowActs({ kind: 'cycle', id: c.id, name: title(c) }, pinned.has(String(c.id)))}>
                    <IconColorPicker label={`Icon and colour of ${title(c)}`} value={{ icon: c.icon ?? 'icon:cycle', color: c.color }} onchange={(v) => patch({ kind: 'cycle', id: c.id, name: title(c) }, { icon: v.icon, color: v.color })} bind:open={() => rowUi.picking === `cycle:${c.id}`, (o) => (rowUi.picking = o ? `cycle:${c.id}` : null)} />
                    <span class="grow"><ListRow selected={opened === c.id} onpick={() => open(opened === c.id ? null : c.id)}>
                                        <span class="title">{title(c)}</span>
                    <span class="dates">{dayText(c.start_date)} to {dayText(c.end_date)}</span>
                    <span class="meter" role="progressbar" aria-label="Progress" aria-valuenow={c.done} aria-valuemax={c.total}><span class="fill" style:width={`${c.total ? (c.done / c.total) * 100 : 0}%`}></span></span>
                    {#if charts && group.band === 'past'}<CycleSpark cycleId={c.id} name={title(c)} />{/if}
                    <span class="progress">{c.done} of {c.total} done</span>
                    {#if group.band === 'current'}<span class="left">{daysLeft(c)} days left</span>{/if}
                  </ListRow></span>
                  </RowActions>
                  {#if group.band === 'current' && next}
                    <Button class="move" onclick={() => (moving = true)}>Move unfinished to next cycle</Button>
                  {/if}
                </li>
                {#if charts && group.band === 'current'}<div class="burnup"><CycleBurnup cycleId={c.id} /></div>{/if}
                {#if opened === c.id}
                  {#key c.id}<CycleRetro cycle={stored.find((x) => x.id === c.id) ?? c} /><div class="issues"><ListView view={cycleView(c)} /></div>{/key}
                {/if}
               {/if}
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    {/each}
  {/if}
</main>

<Dialog bind:open={creating} title="New cycle">
  {#snippet children()}
    <TextInput bind:value={name} label="Cycle name" placeholder="Name (optional)" />
    <TextInput bind:value={start} label="Start date" type="date" />
    <TextInput bind:value={end} label="End date" type="date" />
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (creating = false)}>Cancel</Button>
    <Button variant="primary" onclick={create}>Create cycle</Button>
  {/snippet}
</Dialog>

<Dialog bind:open={moving} title="Move unfinished issues" description={current && next ? `Open issues in ${title(current)} move to ${title(next)}.` : undefined}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (moving = false)}>Cancel</Button>
    <Button variant="primary" onclick={confirmMove}>Move</Button>
  {/snippet}
</Dialog>

<style>
  .fold { display: flex; align-items: center; gap: var(--s-4); height: var(--row-h); color: var(--muted); font-weight: 600; border-bottom: 1px solid var(--border); }
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); }
  .group { display: grid; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: var(--s-5); padding-right: var(--s-4); border-bottom: 1px solid var(--border); }
  .move { flex: none; white-space: nowrap; }
  .title { font-weight: 600; min-width: 120px; }
  .dates, .progress, .left { color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .left { margin-left: auto; color: var(--ink); }
  .meter { width: 120px; height: var(--s-3); border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
  .velocity { max-width: 560px; padding-top: var(--s-5); }
  h3 { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin-bottom: var(--s-3); }
  .burnup { padding: var(--s-5) var(--s-7) var(--s-7); }
  .issues { border-left: 2px solid var(--accent); }
  /* The row's actions take the progress and days-left text's place while the row is hovered or focused. */
  .row:hover .progress, .row:hover .left, .row:focus-within .progress, .row:focus-within .left { visibility: hidden; }
</style>

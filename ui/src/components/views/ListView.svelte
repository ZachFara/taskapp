<script lang="ts">
  import { api } from '../../lib/api';
  import type { Cycle, Issue, IssuePatch, Label, Project, SavedView, Status, StatusCategory } from '../../lib/bindings';
  import { listState } from '../../lib/listState.svelte';
  import { selection } from '../../lib/selection.svelte';
  import { navigate } from '../../app/routes';
  import Checkbox from '../../ui/Checkbox.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import CommandButton from '../CommandButton.svelte';
  import IssueMenu from './IssueMenu.svelte';
  import Menu from '../../ui/Menu.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { workspaceZone, dayText, dueDay as dueDayIn, isOverdue } from './dates';

  // shortcut: row and band heights default to the token values when the CSS is not loaded (tests), upgrade if rows vary in height
  const ROW = 32;
  const BAND = 30;
  const OVERSCAN = 10;
  const WINDOW_ABOVE = 300;
  const FALLBACK_VIEWPORT = 800;
  const PRIORITY_NAMES = ['No priority', 'Low', 'Medium', 'High', 'Urgent'];
  // Backend priority: 0 none, 1 urgent ... 4 low. The glyph counts the other way round.
  const glyph = (p: number) => (p === 0 ? 0 : 5 - p);

  let { view }: { view: SavedView } = $props();

  const storeKey = (id: number) => `cairn.list.collapsed.${id}`;
  type Group = { key: string; kind: 'status' | 'project' | 'cycle' | 'priority' | 'label' | 'none'; ref?: Project | Cycle | Label; name: string; category: StatusCategory; order: number; patch: IssuePatch; rows: Issue[] };
  type Item = { head: Group; collapsed: boolean } | { row: Issue };

  let issues = $state.raw<Issue[]>([]);
  let statuses = $state.raw<Status[]>([]);
  let projects = $state.raw<Project[]>([]);
  let cycles = $state.raw<Cycle[]>([]);
  let labels = $state.raw<Label[]>([]);
  let loaded = $state(false);
  let collapsed = $state<string[]>(readCollapsed(view.id));
  let hovered = $state<number | null>(null);
  let scrollTop = $state(0);
  let height = $state(0);
  let scroller = $state<HTMLElement>();
  let tz = $state('UTC');
  let menu = $state<{ id: number; x: number; y: number } | null>(null);
  let menuOpen = $state(false);
  let draft = $state('');

  function readCollapsed(id: number): string[] {
    try {
      return JSON.parse(localStorage.getItem(storeKey(id)) ?? '[]');
    } catch {
      return [];
    }
  }
  function toggleGroup(key: string) {
    collapsed = collapsed.includes(key) ? collapsed.filter((k) => k !== key) : [...collapsed, key];
    try {
      localStorage.setItem(storeKey(view.id), JSON.stringify(collapsed));
    } catch {
      // storage unavailable: collapse still works for this session
    }
  }

  async function load() {
    const [st, pr, cy, lb, rows, ws] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list(), api.views.query(view.def), workspaceZone(),
    ]);
    tz = ws;
    statuses = st;
    projects = pr;
    cycles = cy;
    labels = lb;
    issues = rows;
    loaded = true;
  }
  $effect(() => {
    view;
    load();
  });

  const groupField = $derived(view.def.group_by?.Fixed ?? null);
  const groups = $derived.by<Group[]>(() => {
    const by = new Map<string, Group>();
    const put = (i: Issue, g: Omit<Group, 'rows'>) => {
      const hit = by.get(g.key) ?? { ...g, rows: [] };
      hit.rows.push(i);
      by.set(g.key, hit);
    };
    const named = <T extends { id: number }>(list: T[], id: number | null, none: string, name: (x: T) => string) => {
      const x = list.find((e) => e.id === id);
      return { name: x ? name(x) : none, order: x ? list.indexOf(x) : Infinity };
    };
    for (const i of issues) {
      const base = { category: 'Unstarted' as StatusCategory, patch: {}, kind: 'none' as Group['kind'] };
      if (groupField === 'Status') {
        const s = statuses.find((x) => x.id === i.status_id);
        put(i, { kind: 'status', key: `s${s?.id ?? 'x'}`, name: s?.name ?? 'Unknown', category: s?.category ?? 'Backlog', order: s?.position ?? Infinity, patch: s ? { status_id: s.id } : {} });
      } else if (groupField === 'Project') {
        put(i, { key: `p${i.project_id}`, ...base, kind: 'project', ref: projects.find((p) => p.id === i.project_id), ...named(projects, i.project_id, 'No project', (p) => p.name), patch: { project_id: i.project_id } });
      } else if (groupField === 'Cycle') {
        put(i, { key: `c${i.cycle_id}`, ...base, kind: 'cycle', ...named(cycles, i.cycle_id, 'No cycle', (c) => c.name ?? `Cycle ${c.number}`), patch: { cycle_id: i.cycle_id } });
      } else if (groupField === 'Priority') {
        put(i, { key: `r${i.priority}`, ...base, kind: 'priority', name: i.priority === 0 ? PRIORITY_NAMES[0] : PRIORITY_NAMES[glyph(i.priority)], order: i.priority === 0 ? 9 : i.priority, patch: { priority: i.priority } });
      } else if (groupField === 'Label') {
        for (const id of i.label_ids.length ? i.label_ids : [null])
          put(i, { key: `l${id}`, ...base, kind: 'label', ref: labels.find((l) => l.id === id), ...named(labels, id, 'No label', (l) => l.name), patch: { label_ids: id === null ? [] : [id] } });
      } else {
        put(i, { key: 'all', ...base, name: '', order: 0 });
      }
    }
    return [...by.values()].sort((a, b) => a.order - b.order);
  });

  const items = $derived.by<Item[]>(() => {
    const out: Item[] = [];
    for (const g of groups) {
      const closed = collapsed.includes(g.key);
      if (g.key !== 'all') out.push({ head: g, collapsed: closed });
      if (!closed) for (const row of g.rows) out.push({ row });
    }
    return out;
  });
  const heightOf = (it: Item) => ('head' in it ? BAND : ROW);
  const tops = $derived.by(() => {
    let y = 0;
    return items.map((it) => ((y += heightOf(it)), y - heightOf(it)));
  });
  const total = $derived(tops.length ? tops[tops.length - 1] + heightOf(items[items.length - 1]) : 0);
  const visibleRows = $derived(items.flatMap((it) => ('row' in it ? [it.row] : [])));
  // Published for the movement commands: the rows the user can see, in order.
  $effect(() => {
    const ids = visibleRows.map((r) => r.id);
    listState.ids = ids;
    selection.setLayout('list');
    selection.setColumns([ids]);
    return () => {
      listState.ids = [];
      selection.setColumns([]);
    };
  });

  const viewport = $derived(height || FALLBACK_VIEWPORT);
  const windowed = $derived(items.length > WINDOW_ABOVE);
  const slice = $derived.by(() => {
    if (!windowed) return items.map((item, k) => ({ item, top: tops[k] }));
    const from = Math.max(0, tops.findIndex((t) => t + ROW > scrollTop) - OVERSCAN);
    const to = Math.min(items.length, (tops.findIndex((t) => t >= scrollTop + viewport) + 1 || items.length) + OVERSCAN);
    return items.slice(from, to).map((item, k) => ({ item, top: tops[from + k] }));
  });

  $effect(() => {
    if (scroller) height = scroller.clientHeight;
  });

  // Keeps the focused row visible, however focus moved.
  $effect(() => {
    const at = items.findIndex((it) => 'row' in it && it.row.id === selection.focused);
    if (at < 0 || !scroller) return;
    if (tops[at] < scroller.scrollTop) scroller.scrollTop = tops[at];
    else if (tops[at] + ROW > scroller.scrollTop + viewport) scroller.scrollTop = tops[at] + ROW - viewport;
  });

  const statusOf = (i: Issue) => statuses.find((s) => s.id === i.status_id);
  const dueDay = (i: Issue) => dueDayIn(i, tz);
  const overdue = (i: Issue) => isOverdue(i, statusOf(i)?.category, tz);
  const estimate = (m: number | null) => (m === null ? '' : m >= 60 ? `${Math.floor(m / 60)}h${m % 60 ? ` ${m % 60}m` : ''}` : `${m}m`);
  const labelsOf = (i: Issue) => i.label_ids.flatMap((id) => labels.find((l) => l.id === id) ?? []);
  const projectOf = (i: Issue) => projects.find((p) => p.id === i.project_id);

  function pick(e: MouseEvent, id: number) {
    if (e.metaKey || e.ctrlKey) {
      selection.toggle(id);
    } else if (e.shiftKey && selection.focused !== null) {
      const ids = visibleRows.map((r) => r.id);
      const [a, b] = [ids.indexOf(selection.focused), ids.indexOf(id)].sort((x, y) => x - y);
      selection.selectAll(ids.slice(a, b + 1));
    } else {
      selection.clear();
    }
    selection.focus(id);
  }

  async function patch(id: number, p: IssuePatch) {
    await api.issues.update(id, p);
    await load();
  }
  async function add(g: Group) {
    const made = await api.issues.create({ title: 'Untitled', ...g.patch });
    await load();
    selection.focus(made.id);
    draft = made.title;
    listState.editing = made.id;
  }
  async function save(id: number) {
    const title = draft.trim();
    listState.editing = null;
    if (title && title !== issues.find((i) => i.id === id)?.title) await patch(id, { title });
  }
  $effect(() => {
    if (listState.editing !== null) draft = issues.find((i) => i.id === listState.editing)?.title ?? '';
  });
  function focusInput(box: HTMLElement) {
    const el = box.querySelector('input');
    el?.focus();
    el?.select();
  }

  function openMenu(id: number, x: number, y: number) {
    menu = { id, x, y };
    menuOpen = true;
  }
</script>

{#snippet lead(g: Group)}
  {#if g.kind === 'priority'}
    <PriorityGlyph level={g.rows[0] ? glyph(g.rows[0].priority) : 0} />
  {:else if g.kind === 'project' || g.kind === 'label'}
    {@const r = g.ref as Project | Label | undefined}
    <EntityIcon icon={(r as Project | undefined)?.icon ?? (g.kind === 'label' ? 'icon:tag' : 'icon:project')} color={r?.color} size={14} />
  {:else}
    <EntityIcon icon="icon:cycle" size={14} />
  {/if}
{/snippet}
{#snippet anchor(props: Record<string, unknown>)}
  <span {...props} class="anchor" style:left="{menu?.x ?? 0}px" style:top="{menu?.y ?? 0}px" aria-hidden="true"></span>
{/snippet}

{#if !loaded}
  <div class="loading" aria-busy="true">
    {#each [0, 1, 2, 3, 4, 5, 6, 7] as k (k)}<Skeleton height="var(--s-7)" width={`${90 - k * 4}%`} />{/each}
  </div>
{:else if issues.length === 0}
  <div class="empty"><EmptyState title="No issues in this view" hint="Issues that match this view will show up here.">
      {#snippet action()}<CommandButton command="issue.create" variant="primary" />{/snippet}
    </EmptyState></div>
{:else}
  <IssueMenu>
  <div class="scroller" role="presentation" bind:this={scroller} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
    <ul class="list" role="list" aria-label={view.name} style:height="{total}px">
      {#each slice as { item, top } (top)}
        {#if 'head' in item}
          <li class="head" style:top="{top}px">
            <GroupHeader
              name={item.head.name}
              count={item.head.rows.length}
              category={item.head.kind === 'status' ? item.head.category : undefined}
              tone={item.head.kind === 'priority' ? (Math.min(4, Math.max(1, item.head.order)) as 1 | 2 | 3 | 4) : 1}
              collapsed={item.collapsed}
              ontoggle={() => toggleGroup(item.head.key)}
              onadd={() => add(item.head)}
            >
              {#snippet leading()}{@render lead(item.head)}{/snippet}
            </GroupHeader>
          </li>
        {:else}
          {@const i = item.row}
          {@const st = statusOf(i)}
          {@const chosen = selection.selected.includes(i.id)}
          <li
            class="row"
            data-key={i.key}
            data-id={i.id}
            data-selected={chosen ? '' : undefined}
            aria-current={selection.focused === i.id ? 'true' : undefined}
            style:top="{top}px"
            onclick={(e) => pick(e, i.id)}
            ondblclick={() => navigate(`/issue/${i.key}`)}
            onmouseenter={() => (hovered = i.id)}
            onmouseleave={() => (hovered = null)}
          >
            <span class="check" role="presentation" onclick={(e) => (e.stopPropagation(), selection.toggle(i.id))}>
              {#if chosen || hovered === i.id}
                <Checkbox checked={chosen} label={`Select ${i.key}`} />
              {/if}
            </span>
            <span class="prio"><PriorityGlyph level={glyph(i.priority)} /></span>
            <span class="key">{i.key}</span>
            <button
              class="status"
              type="button"
              aria-label={`Status ${st?.name ?? 'Unknown'}, change`}
              onclick={(e) => {
                e.stopPropagation();
                selection.focus(i.id);
                const r = e.currentTarget.getBoundingClientRect();
                openMenu(i.id, r.left, r.bottom);
              }}
            >
              <StatusGlyph category={st?.category ?? 'Backlog'} label={st?.name ?? 'Unknown'} />
            </button>
            <span class="title">
              {#if listState.editing === i.id}
                <form onsubmit={(e) => (e.preventDefault(), save(i.id))} onfocusout={() => listState.editing === i.id && save(i.id)}>
                  <span use:focusInput><TextInput bind:value={draft} label={`Title of ${i.key}`} /></span>
                </form>
              {:else}
                <span class="text">{i.title}</span>
              {/if}
            </span>
            <span class="labels">
              {#each labelsOf(i).slice(0, 2) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{/each}
              {#if labelsOf(i).length > 2}<span class="more">+{labelsOf(i).length - 2}</span>{/if}
            </span>
            <span class="project">
              {#if projectOf(i)}{@const p = projectOf(i)!}<EntityIcon icon={p.icon ?? 'icon:project'} color={p.color} size={14} /><span class="pname">{p.name}</span>{/if}
            </span>
            <span class="due" data-overdue={overdue(i) ? '' : undefined}>{dueDay(i) ? dayText(dueDay(i)!) : ''}</span>
            <span class="est">{estimate(i.estimate_minutes)}</span>
          </li>
        {/if}
      {/each}
    </ul>
  </div>
  </IssueMenu>

  <Menu trigger={anchor} bind:open={menuOpen}>
    {#each statuses as s (s.id)}
      <MenuItem label={s.name} icon={s.id === issues.find((x) => x.id === menu?.id)?.status_id ? 'check' : undefined} onSelect={() => patch(menu!.id, { status_id: s.id })} />
    {/each}
  </Menu>
{/if}

<style>
  .loading { display: grid; gap: var(--s-5); padding: var(--s-6) var(--s-7); }
  .empty { padding-top: var(--s-10); }
  .scroller { overflow-y: auto; height: 100%; position: relative; font-size: var(--fs-md); font-variant-numeric: tabular-nums; }
  .list { list-style: none; margin: 0; padding: 0; position: relative; }
  .head, .row { position: absolute; left: 0; right: 0; }
  .row {
    height: var(--row-h);
    display: grid;
    grid-template-columns: var(--s-8) var(--s-8) 64px var(--s-8) minmax(0, 1fr) 150px 140px 56px 56px;
    align-items: center;
    column-gap: var(--s-4);
    padding: 0 var(--s-7) 0 var(--s-4);
    border-bottom: 1px solid var(--border);
    box-shadow: inset 3px 0 0 transparent;
    transition: background var(--dur-1) var(--ease);
  }
  .row:hover { background: var(--raised); }
  .row[data-selected] { background: var(--selection); }
  .row[aria-current='true'] { box-shadow: inset 3px 0 0 var(--signal); background: var(--active); }
  .row[aria-current='true'][data-selected] { background: var(--selection); }
  .check, .prio { display: grid; place-items: center; }
  .key { color: var(--muted); font-weight: 500; }
  .status { border: 0; background: transparent; padding: 0; display: grid; place-items: center; border-radius: var(--r-ctl); }
  .status:hover { background: var(--active); }
  .title { min-width: 0; font-weight: 500; }
  .text { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .labels { display: flex; gap: var(--s-3); overflow: hidden; }
  .more { color: var(--muted); font-size: var(--fs-sm); align-self: center; }
  .project { display: flex; align-items: center; gap: var(--s-3); min-width: 0; color: var(--muted); }
  .pname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .due, .est { text-align: right; color: var(--muted); }
  .due[data-overdue] { color: var(--signal); font-weight: 700; }
  .anchor { position: fixed; width: 0; height: 0; }
</style>

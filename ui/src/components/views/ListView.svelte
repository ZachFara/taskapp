<script lang="ts">
  import { untrack } from 'svelte';
  import { fieldOn } from '../../lib/fields.svelte';
  import { api } from '../../lib/api';
  import type { Cycle, FieldRef, Issue, IssuePatch, Label, Project, SavedView, Sort, Status, StatusCategory } from '../../lib/bindings';
  import { nextSort, sortable } from '../../lib/viewSort';
  import SortHeading from './SortHeading.svelte';
  import { listState } from '../../lib/listState.svelte';
  import { defaultStatusId, issues as issueStore, selectedIssueId } from '../../lib/stores.svelte';
  import { defaultsFromFilter, mergeIssueDefaults } from '../filters/defaults';
  import { selection } from '../../lib/selection.svelte';
  import { navigate } from '../../app/routes';
  import Checkbox from '../../ui/Checkbox.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import Icon from '../../ui/Icon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import { flash } from '../../lib/motion.svelte';
  import CommandButton from '../CommandButton.svelte';
  import IssueMenu from './IssueMenu.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Menu from '../../ui/Menu.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import { priorityGlyph, priorityName, priorityOrder } from '../../lib/priority';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import Tooltip from '../../ui/Tooltip.svelte';
  import { shortcutFor } from '../../lib/commands/registry';
  import TextInput from '../../ui/TextInput.svelte';
  import { dueDay as dueDayIn, isOverdue } from './dates';
  import FieldValue from '../display/FieldValue.svelte';
  import { LEADING, LIST_DEFAULT, density, emptyCtx, fieldName, loadCtx, pointsName, pointsTotal, shown, showEmptyGroups, subIssues, type Ctx } from '../display/types';
  import { onRowsChanged } from './onRowsChanged';
  import { statusRank } from '../../lib/statusOrder';

  // shortcut: row and band heights default to the token values when the CSS is not loaded (tests), upgrade if rows vary in height
  const BAND = 30;
  const OVERSCAN = 10;
  const WINDOW_ABOVE = 300;
  const FALLBACK_VIEWPORT = 800;

  let { view, onsort }: { view: SavedView; onsort?: (field: FieldRef) => void } = $props();
  let localSort = $state.raw<Sort[] | null>(null);
  const activeSort = $derived((localSort ?? view.def.sorts)[0] ?? null);
  function sortHeading(field: FieldRef) {
    if (onsort) onsort(field);
    else localSort = nextSort(localSort, field);
  }

  const storeKey = (id: number) => `cairn.list.collapsed.${id}`;
  type Group = { key: string; kind: 'status' | 'project' | 'cycle' | 'priority' | 'label' | 'points' | 'none'; ref?: Project | Cycle | Label; name: string; category: StatusCategory; icon?: string | null; color?: string | null; order: number; patch: IssuePatch; rows: Issue[] };
  type Item = { head: Group; collapsed: boolean } | { row: Issue; depth: number; kids: number };

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
  let wrapW = $state(0);
  // Not a bind:clientWidth: the page's own tests run where ResizeObserver does not exist.
  function watchWidth(node: HTMLElement) {
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => (wrapW = node.clientWidth));
    ro.observe(node);
    return { destroy: () => ro.disconnect() };
  }
  let ctx = $state.raw<Ctx>(emptyCtx);
  const tz = $derived(ctx.tz);
  const compact = $derived(density(view.def) === 'compact');
  // shortcut: compact height follows the --row-h-compact token value (28), upgrade if the token changes
  const ROW = $derived(compact ? 28 : 32);
  // Wrapped rows differ in height, so the list drops its fixed-height windowing and lets the rows flow.
  const wrap = $derived(view.def.wrap === 'on');
  // Key, priority and status have their own leading columns, so they are not repeated.
  const allFields = $derived(shown(view.def, LIST_DEFAULT).filter((f) => !LEADING.includes(f.Fixed!) && (ctx.points || f.Fixed !== 'Points')));
  let closedParents = $state<number[]>([]);
  const WIDTH: Record<string, string> = { Label: '150px', Project: '140px', DueDate: '56px', Estimate: '72px', Key: '64px', Priority: 'var(--s-8)', Status: '120px', Cycle: '100px', StartDate: '56px', CreatedAt: '56px', UpdatedAt: '56px' };
  const widthOf = (f: FieldRef) => (f.Property === undefined ? (WIDTH[f.Fixed!] ?? '100px') : ({ Select: '120px', MultiSelect: '150px', Checkbox: '56px', Url: '56px', Number: '72px', Date: '72px' } as Record<string, string>)[ctx.defs.find((d) => d.key === f.Property)?.kind ?? ''] ?? '140px');
  const rightAligned = (f: FieldRef) => f.Property === undefined ? ['DueDate', 'StartDate', 'Estimate', 'Points', 'CreatedAt', 'UpdatedAt'].includes(f.Fixed!) : ['Number', 'Date'].includes(ctx.defs.find((d) => d.key === f.Property)?.kind ?? '');
  // Metadata columns drop from the right (lowest priority) once the title would get less than TITLE_MIN.
  const TITLE_MIN = 220;
  const LEAD_W = 32 + 32 + 64 + 32 + 4 * 8 + 2 * 24;
  const px = (f: FieldRef) => parseInt(widthOf(f), 10) || 32;
  const fields = $derived.by(() => {
    if (!wrapW) return allFields;
    let room = wrapW - LEAD_W - TITLE_MIN;
    const keep: FieldRef[] = [];
    for (const f of allFields) {
      room -= px(f) + 8;
      if (room < 0) break;
      keep.push(f);
    }
    return keep;
  });
  // Per-column facts, worked out once rather than for every cell of every row.
  const cells = $derived(fields.map((f) => ({ f, key: f.Property !== undefined ? `P:${f.Property}` : `F:${f.Fixed}`, right: rightAligned(f) })));
  const columns = $derived(`var(--s-8) var(--s-8) 64px var(--s-8) minmax(0, 1fr) ${fields.map(widthOf).join(' ')}`);
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

  let queryVersion = 0;
  async function load() {
    const mine = ++queryVersion;
    const def = $state.snapshot({ ...view.def, sorts: localSort ?? view.def.sorts });
    const [st, pr, cy, lb, rows, c] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list(), api.views.query(def), loadCtx(def),
    ]);
    if (mine !== queryVersion) return;
    ctx = c;
    statuses = st;
    projects = pr;
    cycles = cy;
    labels = lb;
    issues = subIssues(def) === 'hide' ? rows.filter((i) => i.parent_id === null) : rows;
    loaded = true;
  }
  $effect(() => {
    view;
    localSort;
    untrack(load);
  });
  onRowsChanged(() => void load());

  const groupField = $derived(view.def.group_by?.Fixed === 'Points' && !ctx.points ? null : (view.def.group_by?.Fixed ?? null));
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
        put(i, { kind: 'status', key: `s${s?.id ?? 'x'}`, name: s?.name ?? 'Unknown', category: s?.category ?? 'Backlog', icon: s?.icon, color: s?.color, order: s ? statusRank(s) : Infinity, patch: s ? { status_id: s.id } : {} });
      } else if (groupField === 'Project') {
        put(i, { key: `p${i.project_id}`, ...base, kind: 'project', ref: projects.find((p) => p.id === i.project_id), ...named(projects, i.project_id, 'No project', (p) => p.name), patch: { project_id: i.project_id } });
      } else if (groupField === 'Cycle') {
        put(i, { key: `c${i.cycle_id}`, ...base, kind: 'cycle', ...named(cycles, i.cycle_id, 'No cycle', (c) => c.name ?? `Cycle ${c.number}`), patch: { cycle_id: i.cycle_id } });
      } else if (groupField === 'Priority') {
        put(i, { key: `r${i.priority}`, ...base, kind: 'priority', name: priorityName(i.priority), order: priorityOrder(i.priority), patch: { priority: i.priority } });
      } else if (groupField === 'Points') {
        put(i, { key: `pt${i.points}`, ...base, kind: 'points', name: pointsName(i.points), order: i.points ?? Infinity, patch: { points: i.points } });
      } else if (groupField === 'Label') {
        for (const id of i.label_ids.length ? i.label_ids : [null])
          put(i, { key: `l${id}`, ...base, kind: 'label', ref: labels.find((l) => l.id === id), ...named(labels, id, 'No label', (l) => l.name), patch: { label_ids: id === null ? [] : [id] } });
      } else {
        put(i, { key: 'all', ...base, name: '', order: 0 });
      }
    }
    if (showEmptyGroups(view.def)) {
      const empty = (g: Omit<Group, 'rows'>) => by.has(g.key) || by.set(g.key, { ...g, rows: [] });
      const base = { category: 'Unstarted' as StatusCategory, patch: {} };
      if (groupField === 'Status') for (const s of statuses) empty({ key: `s${s.id}`, kind: 'status', name: s.name, category: s.category, icon: s.icon, color: s.color, order: statusRank(s), patch: { status_id: s.id } });
      if (groupField === 'Project') for (const p of projects) empty({ key: `p${p.id}`, ...base, kind: 'project', ref: p, name: p.name, order: projects.indexOf(p), patch: { project_id: p.id } });
      if (groupField === 'Cycle') for (const c of cycles) empty({ key: `c${c.id}`, ...base, kind: 'cycle', name: c.name ?? `Cycle ${c.number}`, order: cycles.indexOf(c), patch: { cycle_id: c.id } });
    }
    return [...by.values()].sort((a, b) => a.order - b.order);
  });

  // Nested: a child follows its parent when both are in the group; a collapsed parent hides its children.
  function ordered(rows: Issue[]): Item[] {
    if (subIssues(view.def) !== 'nest') return rows.map((row) => ({ row, depth: 0, kids: 0 }));
    const ids = new Set(rows.map((r) => r.id));
    const byParent = new Map<number, Issue[]>();
    for (const r of rows) if (r.parent_id !== null) byParent.set(r.parent_id, [...(byParent.get(r.parent_id) ?? []), r]);
    const kidsOf = (id: number) => byParent.get(id) ?? [];
    const out: Item[] = [];
    const walk = (row: Issue, depth: number) => {
      const kids = kidsOf(row.id);
      out.push({ row, depth, kids: kids.length });
      if (!closedParents.includes(row.id)) kids.forEach((k) => walk(k, depth + 1));
    };
    rows.filter((r) => r.parent_id === null || !ids.has(r.parent_id)).forEach((r) => walk(r, 0));
    return out;
  }
  const toggleParent = (id: number) => (closedParents = closedParents.includes(id) ? closedParents.filter((x) => x !== id) : [...closedParents, id]);

  const items = $derived.by<Item[]>(() => {
    const out: Item[] = [];
    for (const g of groups) {
      const closed = collapsed.includes(g.key);
      if (g.key !== 'all') out.push({ head: g, collapsed: closed });
      if (!closed) out.push(...ordered(g.rows));
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
  const windowed = $derived(!wrap && items.length > WINDOW_ABOVE);
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
    if (wrap) {
      scroller.querySelector(`li[data-id="${selection.focused}"]`)?.scrollIntoView({ block: 'nearest' });
      return;
    }
    if (tops[at] < scroller.scrollTop) scroller.scrollTop = tops[at];
    else if (tops[at] + ROW > scroller.scrollTop + viewport) scroller.scrollTop = tops[at] + ROW - viewport;
  });

  const statusOf = (i: Issue) => statuses.find((s) => s.id === i.status_id);
  const dueDay = (i: Issue) => dueDayIn(i, tz);
  const overdue = (i: Issue) => isOverdue(i, statusOf(i)?.category, tz);

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
    const defaults = defaultsFromFilter(view.def.filter, { statuses, projects, propertyDefs: ctx.defs, defaultStatusId: defaultStatusId.value });
    const made = await api.issues.create({ title: 'Untitled', ...mergeIssueDefaults(defaults, g.patch) });
    await load();
    if (!issueStore.some((i) => i.id === made.id)) issueStore.push(made);
    selection.focus(made.id);
    selectedIssueId.value = made.id;
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

  // The command menu runs the same handlers as the group header buttons.
  const focusedGroup = () => groups.find((g) => g.rows.some((r) => r.id === selection.focused));
  $effect(() => {
    const on: [string, () => void][] = [
      ['group.collapse', () => { const g = focusedGroup(); if (g && g.key !== 'all') toggleGroup(g.key); }],
      ['group.collapse.all', () => {
        const keys = groups.filter((g) => g.key !== 'all').map((g) => g.key);
        collapsed = keys.every((k) => collapsed.includes(k)) ? [] : keys;
      }],
      ['group.add', () => { const g = focusedGroup() ?? groups[0]; if (g) void add(g); }],
    ];
    for (const [n, f] of on) addEventListener(`command:${n}`, f);
    return () => on.forEach(([n, f]) => removeEventListener(`command:${n}`, f));
  });

  function openMenu(id: number, x: number, y: number) {
    menu = { id, x, y };
    menuOpen = true;
  }
</script>

{#snippet lead(g: Group)}
  {#if g.kind === 'priority'}
    <PriorityGlyph level={g.rows[0] ? priorityGlyph(g.rows[0].priority) : 0} />
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
  <div class="wrap" use:watchWidth>
  <div class="heads" style:grid-template-columns={columns}>
    <span></span><span>{#if fieldOn('priority')}<SortHeading field={{ Fixed: 'Priority' }} label="Priority" display="P" sort={activeSort} onclick={sortHeading} />{/if}</span><span><SortHeading field={{ Fixed: 'Key' }} label="Key" sort={activeSort} onclick={sortHeading} /></span><span><SortHeading field={{ Fixed: 'Status' }} label="Status" display="S" sort={activeSort} onclick={sortHeading} /></span><span><SortHeading field={{ Fixed: 'Title' }} label="Title" sort={activeSort} onclick={sortHeading} /></span>
    {#each fields as f (f.Property ?? f.Fixed)}<span data-right={rightAligned(f) ? '' : undefined}>{#if sortable(f)}<SortHeading field={f} label={fieldName(f, ctx.defs)} sort={activeSort} onclick={sortHeading} />{:else}{fieldName(f, ctx.defs)}{/if}</span>{/each}
  </div>
  <div class="scroller" role="presentation" bind:this={scroller} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
    <ul class="list" role="grid" aria-multiselectable="true" data-density={compact ? 'compact' : undefined} data-wrap={wrap ? '' : undefined} aria-label={view.name} style:height={wrap ? undefined : `${total}px`}>
      <!-- Keyed by the item, not its position: a row that moves when a group folds keeps its element, so its top eases. -->
      {#each slice as { item, top } ('head' in item ? `h:${item.head.key}` : `r:${item.row.id}`)}
        {#if 'head' in item}
          <li class="head" role="presentation" style:top={wrap ? undefined : `${top}px`}>
            <GroupHeader
              name={item.head.name}
              count={item.head.rows.length}
              total={pointsTotal(item.head.rows, shown(view.def, LIST_DEFAULT), ctx)}
              category={item.head.kind === 'status' ? item.head.category : undefined}
              icon={item.head.icon}
              color={item.head.color}
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
            role="row"
            use:flash={{ id: i.id, sig: `${i.status_id}:${i.priority}`, key: i.key }}
            aria-selected={chosen}
            data-key={i.key}
            data-id={i.id}
            data-depth={item.depth ? item.depth : undefined}
            data-selected={chosen ? '' : undefined}
            aria-current={selection.focused === i.id ? 'true' : undefined}
            style:top={wrap ? undefined : `${top}px`}
            style:grid-template-columns={columns}
            style:height={compact && !wrap ? 'var(--row-h-compact)' : undefined}
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
            <span class="prio">{#if fieldOn('priority')}<PriorityGlyph level={priorityGlyph(i.priority)} />{/if}</span>
            <span class="key">{i.key}</span>
            {#snippet statusBtn(tp: Record<string, unknown>)}
              <button
                {...tp}
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
                <StatusGlyph category={st?.category ?? 'Backlog'} icon={st?.icon} color={st?.color} label={st?.name ?? 'Unknown'} />
              </button>
            {/snippet}
            <!-- shortcut: only the hovered or focused row carries a Tooltip (one per row costs ~30% at 1000 rows), upgrade to one delegated tooltip -->
            {#if hovered === i.id || selection.focused === i.id}
              <Tooltip label="Status" shortcut={shortcutFor('issue.status')}>{#snippet children(tp)}{@render statusBtn(tp)}{/snippet}</Tooltip>
            {:else}
              {@render statusBtn({})}
            {/if}
            <span class="title" style:padding-left={item.depth ? `calc(${item.depth} * var(--s-6))` : undefined}>
              {#if item.kids}
                {@const open = !closedParents.includes(i.id)}
                <button type="button" class="disc" data-open={open ? '' : undefined} aria-label={open ? 'Collapse sub-issues' : 'Expand sub-issues'} onclick={(e) => (e.stopPropagation(), toggleParent(i.id))}><Icon name="chevron-right" size={14} /></button>
              {/if}
              {#if listState.editing === i.id}
                <form onsubmit={(e) => (e.preventDefault(), save(i.id))} onfocusout={() => listState.editing === i.id && save(i.id)}>
                  <span use:focusInput><TextInput bind:value={draft} label={`Title of ${i.key}`} /></span>
                </form>
              {:else}
                <span class="text">{i.title}</span>
              {/if}
            </span>
            {#each cells as { f, key, right } (key)}
              <span class="cell" data-field={key} data-right={right ? '' : undefined} data-overdue={f.Fixed === 'DueDate' && overdue(i) ? '' : undefined}>
                <FieldValue field={f} issue={i} {ctx} all={wrap} />
              </span>
            {/each}
            <!-- shortcut: only the hovered or focused row renders its actions (a tooltip per row costs the 1000-row budget) -->
            {#if hovered === i.id || selection.focused === i.id}
              <span class="hover-acts">
                <!-- The "…" opens the issue menu that right-click opens: the menu's own listener on the list picks the row. -->
                <IconButton icon="more" label={`More actions for ${i.key}`} onclick={(e) => {
                  e.stopPropagation();
                  const r = e.currentTarget.getBoundingClientRect();
                  e.currentTarget.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.left, clientY: r.bottom }));
                }} />
              </span>
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  </div>
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
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .heads {
    display: grid; align-items: center; column-gap: var(--s-4); flex: none; height: var(--row-h-compact); padding: 0 var(--s-7) 0 var(--s-4);
    border-bottom: 1px solid var(--border); color: var(--muted); font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase;
  }
  .heads span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .heads [data-right] { text-align: right; }
  .scroller { overflow-y: auto; flex: 1; min-height: 0; position: relative; font-size: var(--fs-md); font-variant-numeric: tabular-nums; }
  .list { list-style: none; margin: 0; padding: 0; position: relative; }
  .head, .row { position: absolute; left: 0; right: 0; }
  .head { transition: top var(--dur-3) var(--ease); }
  .row {
    height: var(--row-h);
    display: grid;
    align-items: center;
    column-gap: var(--s-4);
    padding: 0 var(--s-7) 0 var(--s-4);
    border-bottom: 1px solid var(--border);
    box-shadow: inset 3px 0 0 transparent;
    transition: background var(--dur-1) var(--ease), top var(--dur-3) var(--ease);
  }
  .row:hover { background: var(--raised); }
  /* Overlays the row's right end, so nothing shifts when it appears. */
  .hover-acts { position: absolute; right: var(--s-7); display: flex; gap: var(--s-1); background: var(--raised); }
  .row[data-selected] { background: var(--selection); }
  .row[aria-current='true'] { box-shadow: inset 3px 0 0 var(--signal); background: var(--active); }
  .row[aria-current='true'][data-selected] { background: var(--selection); }
  .check, .prio { display: grid; place-items: center; }
  .key { color: var(--muted); font-weight: 500; }
  .status { border: 0; background: transparent; padding: 0; display: grid; place-items: center; border-radius: var(--r-ctl); }
  .status:hover { background: var(--active); }
  .title { min-width: 0; font-weight: 500; display: flex; align-items: center; gap: var(--s-2); }
  .disc { display: grid; place-items: center; border: 0; background: transparent; padding: 0; color: var(--muted); border-radius: var(--r-ctl); transition: transform var(--dur-1) var(--ease); }
  .disc[data-open] { transform: rotate(90deg); }
  .text { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cell { min-width: 0; display: flex; align-items: center; color: var(--muted); overflow: hidden; }
  .cell[data-right] { justify-content: flex-end; }
  .cell[data-overdue] { color: var(--signal); font-weight: 700; }
  .anchor { position: fixed; width: 0; height: 0; }
  /* Wrapped lists: rows take the height their labels and titles need, never less than a normal row. */
  .list[data-wrap] .head, .list[data-wrap] .row { position: relative; height: auto; min-height: var(--row-h); padding-block: var(--s-2); }
  .list[data-wrap][data-density] .row { min-height: var(--row-h-compact); }
  .list[data-wrap] .text { white-space: normal; overflow-wrap: anywhere; }
  .list[data-wrap] .cell { overflow: visible; }
  .list[data-wrap] :global(.chips) { flex-wrap: wrap; overflow: visible; }
</style>

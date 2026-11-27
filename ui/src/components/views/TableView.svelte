<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import type { FieldRef, FixedField, Issue, IssuePatch, PropertyKind, SavedView, StatusCategory, Value } from '../../lib/bindings';
  import { selection } from '../../lib/selection.svelte';
  import { currentViewId, issues, propertyDefs, refreshAll, statuses, views } from '../../lib/stores.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import CommandButton from '../CommandButton.svelte';
  import IssueMenu from './IssueMenu.svelte';
  import { api } from '../../lib/api';
  import type { Label, Project } from '../../lib/bindings';
  import { workspaceZone, dayText, dueDay, isOverdue } from './dates';
  import EmptyState from '../../ui/EmptyState.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import { flash } from '../../lib/motion.svelte';
  import IconButton from '../../ui/IconButton.svelte';
    import { fixedOn } from '../../lib/fields.svelte';
    import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
    import { priorityGlyph, priorityName, priorityOrder } from '../../lib/priority';
  import Select from '../../ui/Select.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import Button from '../../ui/Button.svelte';
  import Popover from '../../ui/Popover.svelte';
  import DisplayPanel from '../display/DisplayPanel.svelte';
  import FieldValue from '../display/FieldValue.svelte';
  import { emptyCtx, loadCtx, pointsTotal, pointsName, type Ctx, type DisplayDef } from '../display/types';
  import { onRowsChanged } from './onRowsChanged';
  import { sortable } from '../../lib/viewSort';
  import SortHeading from './SortHeading.svelte';

  let { view, onsort }: { view: SavedView; onsort?: (field: FieldRef) => void } = $props();

  type EditKind = PropertyKind | 'Status' | 'Priority' | 'Project' | 'Labels' | 'Read';
  // Fixed fields a table can show and edit. Others are left out of the column picker and skipped in views.
  const FIXED: Partial<Record<FixedField, { label: string; kind: EditKind }>> = {
    Title: { label: 'Title', kind: 'Text' },
    Status: { label: 'Status', kind: 'Status' },
    Priority: { label: 'Priority', kind: 'Priority' },
    StartDate: { label: 'Start', kind: 'Date' },
    DueDate: { label: 'Due', kind: 'Date' },
    Estimate: { label: 'Estimate', kind: 'Number' },
    Points: { label: 'Points', kind: 'Read' },
    Project: { label: 'Project', kind: 'Project' },
    Label: { label: 'Labels', kind: 'Labels' },
    // Shown through FieldValue and not editable here.
    Key: { label: 'Key', kind: 'Read' }, Milestone: { label: 'Milestone', kind: 'Read' }, Cycle: { label: 'Cycle', kind: 'Read' },
    Parent: { label: 'Parent', kind: 'Read' }, SubIssueProgress: { label: 'Sub-issues', kind: 'Read' }, Checklist: { label: 'Checklist', kind: 'Read' }, RelationCount: { label: 'Relations', kind: 'Read' },
    CommentCount: { label: 'Comments', kind: 'Read' }, CreatedAt: { label: 'Created', kind: 'Read' }, UpdatedAt: { label: 'Updated', kind: 'Read' },
  };
  // A view with no columns of its own shows these.
  const DEFAULT: FieldRef[] = ['Priority', 'Title', 'Status', 'Project', 'Label', 'DueDate', 'Estimate', 'Checklist'].map((f) => ({ Fixed: f as FixedField }));
  const PATCH: Partial<Record<FixedField, (v: Value) => IssuePatch>> = {
    Title: (v) => ({ title: v as string }),
    Status: (v) => ({ status_id: v as number }),
    Priority: (v) => ({ priority: v as number }),
    StartDate: (v) => ({ start_date: v as string | null }),
    DueDate: (v) => ({ due: v === null ? null : { Date: v as string } }),
    Estimate: (v) => ({ estimate_minutes: v as number | null }),
  };

  const colId = (ref: FieldRef) => (ref.Property !== undefined ? `prop:${ref.Property}` : `fixed:${ref.Fixed}`);
  const kindOf = (ref: FieldRef): EditKind | null =>
    ref.Property !== undefined
      ? (propertyDefs.find((d) => d.key === ref.Property)?.kind ?? null)
      : (FIXED[ref.Fixed!]?.kind ?? null);

  // The store copy is newer after a column save; the prop is the fallback before the first refresh.
  const live = $derived(views.find((v) => v.id === view.id) ?? view);
  const def = $derived(live.def);
  // Columns whose property def is gone (or fixed field not supported) are skipped, keeping their index in the def.
  const base = $derived(def.columns.length ? def.columns : DEFAULT);
  // Points is invisible when the estimation setting excludes it: a stored Points column or group is ignored, not rewritten.
  const groupBy = $derived(def.group_by?.Fixed === 'Points' && !ctx.points ? null : def.group_by);
  const cols = $derived(
    base
      .filter((ref) => ctx.points || ref.Fixed !== 'Points')
      .map((ref, index) => ({ ref, index, id: colId(ref), kind: kindOf(ref), label: label(ref) }))
      .filter((c): c is typeof c & { kind: EditKind } => c.kind !== null && fixedOn(c.ref.Fixed)),
  );
  function label(ref: FieldRef) {
    return ref.Property !== undefined ? (propertyDefs.find((d) => d.key === ref.Property)?.name ?? '') : FIXED[ref.Fixed!]!.label;
  }
  let editing = $state<{ row: number; col: number } | null>(null);
  let draft = $state('');
  let original = $state('');
  let error = $state<string | null>(null);
  let activeCol = $state(0);
  let sort = $state<{ col: string; dir: 'asc' | 'desc' } | null>(null);
  const activeSort = $derived(sort ?? (onsort && view.def.sorts[0] ? { col: colId(view.def.sorts[0].field), dir: view.def.sorts[0].descending ? 'desc' as const : 'asc' as const } : null));
  $effect(() => {
    view.def.sorts;
    if (onsort) sort = null;
  });
  let widths = $state<Record<string, number>>({});
  let collapsed = $state(new Set<string>());
  let loaded = $state(false);
  let picked = $state('');
  let editorEl = $state<HTMLElement>();
  let projects = $state.raw<Project[]>([]);
  let labels = $state.raw<Label[]>([]);
  let tz = $state('UTC');
  let ctx = $state.raw<Ctx>(emptyCtx);
  let tableIssues = $state.raw<Issue[]>([]);
  let queryVersion = 0;
  async function loadRows(d: typeof view.def) {
    const mine = ++queryVersion;
    const rows = await api.views.query(d);
    if (mine === queryVersion) tableIssues = rows;
  }
  $effect(() => {
    const d = $state.snapshot(view.def);
    issues.map((issue) => issue.id);
    untrack(() => void loadRows(d));
  });
  onRowsChanged(() => void loadRows($state.snapshot(view.def)));
  $effect(() => {
    const d = $state.snapshot(def);
    loadCtx(d).then((c) => (ctx = c));
  });

  onMount(() => {
    currentViewId.value = view.id;
    void refreshAll().finally(() => (loaded = true));
    void Promise.all([api.projects.list(), api.labels.list(), workspaceZone()]).then(([p, l, z]) => ([projects, labels, tz] = [p, l, z]));
  });

  const raw = (i: Issue, ref: FieldRef): Value => {
    if (ref.Property !== undefined) return i.properties[ref.Property] ?? null;
    switch (ref.Fixed) {
      case 'Key': return Number(i.key.slice(i.key.lastIndexOf('-') + 1));
      case 'Title': return i.title;
      case 'Status': return i.status_id;
      case 'Priority': return i.priority;
      case 'StartDate': return i.start_date;
      case 'DueDate': return dueDay(i, tz);
      case 'Project': return i.project_id;
      case 'Label': return i.label_ids.join(',');
      case 'Estimate': return i.estimate_minutes;
      case 'Points': return i.points;
      default: return null;
    }
  };
  const optionsOf = (kind: EditKind, ref: FieldRef) =>
    kind === 'Status'
      ? statuses.map((s) => ({ value: String(s.id), label: s.name, glyph: { category: s.category, icon: s.icon, color: s.color } }))
      : ref.Property !== undefined
        ? (propertyDefs.find((d) => d.key === ref.Property)?.options ?? []).map((o) => ({ value: o.value, label: o.value }))
        : [];
  const draftOf = (v: Value, kind: EditKind) => (v === null ? '' : kind === 'Status' ? String(v) : Array.isArray(v) ? v.join(', ') : String(v));

  // Sorting is per visit and stays client-side; the query order is the base order.
  const sortValue = (i: Issue, ref: FieldRef): Value => {
    if (ref.Fixed === 'Status') return statuses.find((s) => s.id === i.status_id)?.position ?? null;
    return raw(i, ref);
  };
  const isEmpty = (v: Value) => v === null || v === '';
  function compare(a: Value, b: Value): number {
    if (typeof a === 'number' && typeof b === 'number') return a - b;
    if (typeof a === 'boolean' && typeof b === 'boolean') return Number(a) - Number(b);
    return String(a).localeCompare(String(b));
  }
  const sorted = $derived.by(() => {
    const s = sort;
    const col = s && (s.col === 'fixed:Key' ? { ref: { Fixed: 'Key' as const } } : cols.find((c) => c.id === s.col));
    if (!s || !col) return tableIssues;
    const sign = s.dir === 'asc' ? 1 : -1;
    return [...tableIssues].sort((x, y) => {
      const a = sortValue(x, col.ref);
      const b = sortValue(y, col.ref);
      // Empty cells sort last in both directions.
      if (isEmpty(a) || isEmpty(b)) return Number(isEmpty(a)) - Number(isEmpty(b));
      return sign * compare(a, b);
    });
  });

  type Group = { name: string; category: StatusCategory; status: boolean; icon?: string | null; color?: string | null; priority?: number; rows: Issue[] };
  const groupOf = (i: Issue): { name: string; category: StatusCategory; status: boolean; icon?: string | null; color?: string | null; priority?: number } => {
    const field = groupBy;
    if (field?.Fixed === 'Status') {
      const st = statuses.find((s) => s.id === i.status_id);
      return { name: st?.name ?? 'Unknown', category: st?.category ?? 'Backlog', status: true, icon: st?.icon, color: st?.color };
    }
    if (field?.Fixed === 'Points') return { name: pointsName(i.points), category: 'Backlog' as StatusCategory, status: false };
    if (field?.Fixed === 'Priority') return { name: priorityName(i.priority), category: 'Backlog' as StatusCategory, status: false, priority: i.priority };
    if (field?.Property !== undefined) {
      const v = i.properties[field.Property];
      return { name: isEmpty(v ?? null) ? 'None' : String(v), category: 'Backlog' as StatusCategory, status: false };
    }
    return { name: '', category: 'Backlog' as StatusCategory, status: true };
  };
  // Groups keep the order of the rows; status groups follow the status positions.
  const groups = $derived.by<Group[]>(() => {
    const byName = new Map<string, Group & { order: number }>();
    for (const i of sorted) {
      const g = groupOf(i);
      const hit = byName.get(g.name) ?? { ...g, rows: [], order: groupOrder(i) };
      hit.rows.push(i);
      byName.set(g.name, hit);
    }
    if ((def as DisplayDef).show_empty_groups && groupBy?.Fixed === 'Status')
      for (const st of statuses) if (!byName.has(st.name)) byName.set(st.name, { name: st.name, category: st.category, status: true, icon: st.icon, color: st.color, rows: [], order: st.position });
    return [...byName.values()].sort((a, b) => a.order - b.order);
  });
  function groupOrder(i: Issue): number {
    const field = groupBy;
    if (field?.Fixed === 'Status') return statuses.find((s) => s.id === i.status_id)?.position ?? Infinity;
    if (field?.Fixed === 'Priority') return priorityOrder(i.priority);
    if (field?.Fixed === 'Points') return i.points ?? Infinity;
    return 0;
  }
  const visibleRows = $derived(groups.flatMap((g) => (collapsed.has(g.name) ? [] : g.rows)));
  const visibleIds = $derived(visibleRows.map((i) => i.id));
  const activeRowId = $derived(selection.focused ?? visibleIds[0] ?? null);

  function startEdit(issue: Issue, c: number) {
    const col = cols[c];
    if (!col || col.kind === 'Project' || col.kind === 'Labels' || col.kind === 'Read') return;
    if (col.kind === 'Checkbox') return void save(issue, col, raw(issue, col.ref) !== true);
    editing = { row: visibleIds.indexOf(issue.id), col: c };
    original = draft = draftOf(raw(issue, col.ref), col.kind);
    error = null;
  }
  const editingIssue = () => (editing ? visibleRows[editing.row] : undefined);

  type Parsed = { ok: true; value: Value } | { ok: false; message: string };
  // Client-side checks run before the API call; the API's own errors are shown as they come.
  function parse(draft: string, col: { ref: FieldRef; kind: EditKind }): Parsed {
    const t = draft.trim();
    const isTitle = col.ref.Fixed === 'Title';
    switch (col.kind) {
      case 'Text':
      case 'Url':
      case 'NoteLink':
        if (isTitle && t === '') return { ok: false, message: 'Title cannot be empty' };
        return { ok: true, value: t === '' ? null : draft };
      case 'Number': {
        if (t === '') return { ok: true, value: null };
        const n = Number(t);
        return Number.isFinite(n) ? { ok: true, value: n } : { ok: false, message: `"${draft}" is not a number` };
      }
      case 'Priority': {
        const n = Number(t);
        return Number.isInteger(n) && n >= 0 && n <= 4 ? { ok: true, value: n } : { ok: false, message: 'Priority must be 0 to 4' };
      }
      case 'Date':
        if (t === '') return { ok: true, value: null };
        return /^\d{4}-\d{2}-\d{2}$/.test(t) ? { ok: true, value: t } : { ok: false, message: 'Use YYYY-MM-DD' };
      case 'Status': {
        const id = Number(t);
        return statuses.some((s) => s.id === id) ? { ok: true, value: id } : { ok: false, message: 'Pick a status' };
      }
      case 'Select': {
        if (t === '') return { ok: true, value: null };
        const allowed = optionsOf('Select', col.ref).map((o) => o.value);
        return allowed.includes(t) ? { ok: true, value: t } : { ok: false, message: `Pick one of: ${allowed.join(', ')}` };
      }
      case 'MultiSelect': {
        const picks = t === '' ? [] : t.split(',').map((x) => x.trim()).filter(Boolean);
        const allowed = optionsOf('MultiSelect', col.ref).map((o) => o.value);
        const bad = picks.filter((p) => !allowed.includes(p));
        return bad.length ? { ok: false, message: `Not an option: ${bad.join(', ')}` } : { ok: true, value: picks };
      }
      default:
        return { ok: true, value: null };
    }
  }

  async function save(issue: Issue, col: { ref: FieldRef; kind: EditKind }, value: Value) {
    try {
      if (col.ref.Property !== undefined) await api.properties.setValue(issue.id, col.ref.Property, value);
      else await api.issues.update(issue.id, PATCH[col.ref.Fixed!]!(value));
      await refreshAll();
      error = null;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function commit() {
    const issue = editingIssue();
    if (!editing || !issue) return;
    const col = cols[editing.col];
    const parsed = parse(draft, col);
    if (!parsed.ok) {
      error = parsed.message;
      return;
    }
    editing = null;
    await save(issue, col, parsed.value);
  }

  // Pickers commit as soon as the value changes; text and date fields commit on Enter (the form's submit).
  const pickerKind = (kind: EditKind | undefined) => kind === 'Status' || kind === 'Select';
  $effect(() => {
    const col = editing ? cols[editing.col] : undefined;
    if (col && pickerKind(col.kind) && draft !== original) untrack(() => void commit());
  });
  $effect(() => {
    if (editing && editorEl) void tick().then(() => editorEl?.querySelector<HTMLElement>('input, button')?.focus());
  });

  function pick(issue: Issue, c: number) {
    if (editing) {
      if (editing.col === c && visibleIds[editing.row] === issue.id) return;
      void commit();
      if (editing) return;
    }
    selection.focus(issue.id);
    activeCol = c;
  }

  // j/k move through the rows the shared selection holds, so the table publishes them and its layout.
  $effect(() => {
    selection.setLayout('table');
    selection.setColumns([visibleIds]);
    return () => selection.setColumns([]);
  });
  // The key commands arrive as window events; each acts on the table's own state.
  function moveCol(delta: number) {
    activeCol = Math.min(Math.max(activeCol + delta, 0), Math.max(cols.length - 1, 0));
  }
  function editActive() {
    const issue = visibleRows.find((i) => i.id === activeRowId);
    if (issue && !editing) startEdit(issue, activeCol);
  }
  function cancel() {
    if (editing) {
      editing = null;
      error = null;
    } else selection.clear();
  }
  function editField(field: FixedField) {
    const c = cols.findIndex((x) => x.ref.Fixed === field);
    if (editing || c < 0) return;
    activeCol = c;
    editActive();
  }
  $effect(() => {
    const handlers: Record<string, () => void> = {
      left: () => moveCol(-1),
      right: () => moveCol(1),
      edit: editActive,
      cancel,
      status: () => editField('Status'),
      priority: () => editField('Priority'),
    };
    const listeners = Object.entries(handlers).map(([id, fn]) => {
      const name = `command:table.${id}`;
      addEventListener(name, fn);
      return [name, fn] as const;
    });
    return () => listeners.forEach(([name, fn]) => removeEventListener(name, fn));
  });

  async function saveColumns(columns: FieldRef[]) {
    try {
      await api.views.save(view.id, live.name, { ...def, columns });
      await refreshAll();
      error = null;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  // Every Display panel change is saved with the rest of the view.
  async function changeDisplay(patch: Partial<DisplayDef>) {
    try {
      await api.views.save(view.id, live.name, { ...def, ...patch });
      await refreshAll();
      error = null;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  function moveColumn(index: number, delta: number) {
    const columns = [...base];
    const target = index + delta;
    if (target < 0 || target >= columns.length) return;
    [columns[index], columns[target]] = [columns[target], columns[index]];
    void saveColumns(columns);
  }

  const removeColumn = (index: number) => void saveColumns(base.filter((_, i) => i !== index));

  function cycleSort(id: string) {
    const col = id === 'fixed:Key' ? { ref: { Fixed: 'Key' as const } } : cols.find((c) => c.id === id);
    if (onsort && col && sortable(col.ref)) {
      sort = null;
      onsort(col.ref);
      return;
    }
    sort = sort?.col !== id ? { col: id, dir: 'asc' } : sort.dir === 'asc' ? { col: id, dir: 'desc' } : null;
  }

  // Dragging the grip sets the width; widths last for the visit only.
  function startResize(e: PointerEvent, id: string) {
    const th = (e.currentTarget as HTMLElement).closest('th')!;
    const startX = e.clientX;
    const startW = th.getBoundingClientRect().width;
    const move = (ev: PointerEvent) => (widths = { ...widths, [id]: Math.max(80, startW + ev.clientX - startX) });
    const up = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
  }

  const toggleGroup = (name: string) => {
    const next = new Set(collapsed);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    collapsed = next;
  };
  const rowSelected = (i: Issue) => selection.selected.includes(i.id);
  const grouped = $derived(groupBy !== null);
</script>

<section class="table-view">
  {#if error}
    <p role="alert" class="error">{error}</p>
  {/if}

  {#if loaded && issues.length === 0}
    <EmptyState title="No issues in this view" hint="Add an issue, or change the view's filter.">
      {#snippet action()}<CommandButton command="issue.create" variant="primary" />{/snippet}
    </EmptyState>
  {:else if !loaded}
    <div class="loading" aria-busy="true">
      {#each [0, 1, 2, 3] as n (n)}<Skeleton height="var(--row-h)" />{/each}
    </div>
  {:else}
    <IssueMenu>
    <table role="grid" class:grouped data-wrap={view.def.wrap === 'on' ? '' : undefined}>
      <colgroup>
        <col class="key-col" />
        {#each cols as col (col.id)}
          <col style:width={widths[col.id] ? `${widths[col.id]}px` : undefined} />
        {/each}
      </colgroup>
      <thead>
        <tr>
          <th scope="col" class="key-head" aria-sort={activeSort?.col === 'fixed:Key' ? (activeSort.dir === 'asc' ? 'ascending' : 'descending') : undefined}><SortHeading field={{ Fixed: 'Key' }} label="Key" sort={activeSort?.col === 'fixed:Key' ? { field: { Fixed: 'Key' }, descending: activeSort.dir === 'desc' } : null} onclick={() => cycleSort('fixed:Key')} /></th>
          {#each cols as col (col.id)}
            <th scope="col" data-col={col.id} data-sticky={col.id === 'fixed:Title' ? '' : undefined} class:num={col.kind === 'Number'} aria-sort={activeSort?.col === col.id ? (activeSort.dir === 'asc' ? 'ascending' : 'descending') : undefined}>
              <span class="head">
                <Button variant="quiet" icon={activeSort?.col === col.id ? (activeSort.dir === 'asc' ? 'chevron-up' : 'chevron-down') : undefined} onclick={() => cycleSort(col.id)}>{col.label}</Button>
                <span class="tools">
                <span class="flip"><IconButton icon="chevron-right" label="Move {col.label} left" onclick={() => moveColumn(col.index, -1)} /></span>
                <IconButton icon="chevron-right" label="Move {col.label} right" onclick={() => moveColumn(col.index, 1)} />
                <IconButton icon="close" label="Remove column {col.label}" onclick={() => removeColumn(col.index)} />
                </span>
              </span>
              <span class="grip" role="separator" aria-orientation="vertical" aria-label="Resize {col.label}" onpointerdown={(e) => startResize(e, col.id)}></span>
            </th>
          {/each}
          <th scope="col" class="add-col">
            <Popover>
              {#snippet trigger(props)}<span {...props} class="trig"><IconButton icon="plus" label="Add field" /></span>{/snippet}
              <DisplayPanel {def} onchange={changeDisplay} />
            </Popover>
          </th>
        </tr>
      </thead>
      {#each groups as g (g.name)}
        <tbody data-group={g.name}>
          {#if grouped}
            <tr class="band-row">
              <td colspan={cols.length + 1}>
                <GroupHeader name={g.name || 'None'} count={g.rows.length} total={pointsTotal(g.rows, cols.map((c) => c.ref), ctx)} category={g.status ? g.category : undefined} icon={g.icon} color={g.color} tone={g.priority ? (Math.min(4, Math.max(1, g.priority)) as 1 | 2 | 3 | 4) : 1} collapsed={collapsed.has(g.name)} ontoggle={() => toggleGroup(g.name)}>
                  {#snippet leading()}{#if g.priority !== undefined}<PriorityGlyph level={priorityGlyph(g.priority)} />{/if}{/snippet}
                </GroupHeader>
              </td>
            </tr>
          {/if}
          {#if !collapsed.has(g.name)}
            {#each g.rows as issue (issue.id)}
              <tr use:flash={{ id: issue.id, sig: `${issue.status_id}:${issue.priority}`, key: issue.key }} data-key={issue.key} data-id={issue.id} aria-selected={rowSelected(issue)} data-focused={issue.id === activeRowId ? '' : undefined}>
                <th scope="row" class="key-cell">{issue.key}</th>
                {#each cols as col, c (col.id)}
                  {@const isEditing = editing !== null && visibleIds[editing.row] === issue.id && editing.col === c}
                  <td
                    role="gridcell"
                    data-col={col.id}
                    data-sticky={col.id === 'fixed:Title' ? '' : undefined}
                    data-kind={col.kind}
                    class:num={col.kind === 'Number'}
                    data-active={issue.id === activeRowId && c === activeCol ? 'true' : undefined}
                    onclick={() => pick(issue, c)}
                  >
                    {#if isEditing}
                      <form class="editor" bind:this={editorEl} onsubmit={(e) => { e.preventDefault(); void commit(); }}>
                        {#if col.kind === 'Status'}
                          <Select label={col.label} items={optionsOf(col.kind, col.ref)} bind:value={draft} />
                        {:else if col.kind === 'Select'}
                          <Select label={col.label} items={optionsOf(col.kind, col.ref)} bind:value={draft} />
                        {:else if col.kind === 'Date'}
                          <TextInput label={col.label} type="date" bind:value={draft} invalid={error !== null} />
                        {:else if col.kind === 'Number'}
                          <TextInput label={col.label} bind:value={draft} invalid={error !== null} />
                        {:else}
                          <TextInput label={col.label} bind:value={draft} invalid={error !== null} placeholder={col.kind === 'MultiSelect' ? 'Comma separated' : undefined} />
                        {/if}
                      </form>
                    {:else if col.kind === 'Checkbox'}
                      <Checkbox label={`${col.label} for ${issue.key}`} bind:checked={() => raw(issue, col.ref) === true, (v) => void save(issue, col, v)} />
                    {:else}
                      <span class="due" data-overdue={col.ref.Fixed === 'DueDate' && isOverdue(issue, statuses.find((s) => s.id === issue.status_id)?.category, tz) ? '' : undefined}>
                        <FieldValue field={col.ref} {issue} {ctx} all={view.def.wrap === 'on'} />
                      </span>
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          {/if}
        </tbody>
      {/each}
    </table>
    </IssueMenu>
  {/if}
</section>

<style>
  .table-view {
    display: grid;
    gap: var(--s-4);
    padding: 0;
    min-width: 0;
    background: var(--sheet);
    color: var(--ink);
    font-size: var(--fs-md);
  }
  /* Header labels are quiet text on the sheet; the "+" stays in view at the right edge. */
  .head :global(.btn) { border-color: transparent; background: transparent; padding: 0 var(--s-2); font: inherit; letter-spacing: inherit; text-transform: inherit; }
  /* Key and Title stay put while the rest scrolls sideways; the "+" is the last column, never laid over one. */
  .add-col { width: var(--s-10); text-align: end; }
  .key-col, .key-head, .key-cell { width: calc(var(--s-10) * 2); min-width: calc(var(--s-10) * 2); box-sizing: border-box; }
  [data-sticky] { position: sticky; left: calc(var(--s-10) * 2); background: var(--sheet); z-index: 1; box-shadow: 1px 0 0 var(--border); }
  thead [data-sticky] { z-index: calc(var(--z-sticky) + 1); }
  .trig { display: inline-flex; }
  .due[data-overdue] { color: var(--signal); font-weight: 700; }
  th.num { text-align: end; }
  .loading {
    display: grid;
    gap: var(--s-3);
  }
  .error {
    margin: 0;
    color: var(--signal);
    font-weight: 600;
  }
  table {
    width: max-content;
    min-width: 100%;
    border-collapse: separate;
    border-spacing: 0;
  }
  thead th {
    position: sticky;
    top: 0;
    z-index: var(--z-sticky);
    height: var(--row-h);
    padding: 0 var(--s-5);
    background: var(--sheet);
    color: var(--muted);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-xs);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    text-align: start;
    white-space: nowrap;
  }
  .head {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
  }
  .flip {
    display: inline-grid;
    transform: scaleX(-1);
  }
  .grip {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: var(--s-3);
    cursor: col-resize;
  }
  .key-head {
    left: 0;
    z-index: calc(var(--z-sticky) + 1);
  }
  .tools { display: inline-flex; opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  th:hover .tools, th:focus-within .tools { opacity: 1; }
  td {
    vertical-align: middle;
    height: var(--row-h);
    padding: 0 var(--s-5);
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Wrapped tables: cells keep their row height as a minimum and grow for several labels or a long title. */
  table[data-wrap] td { white-space: normal; overflow: visible; padding-block: var(--s-2); }
  table[data-wrap] td :global(.chips) { flex-wrap: wrap; overflow: visible; }
  table[data-wrap] td :global(.text) { white-space: normal; overflow-wrap: anywhere; }
  td.num {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }
  .key-cell {
    position: sticky;
    left: 0;
    z-index: 1;
    padding: 0 var(--s-5);
    background: var(--sheet);
    color: var(--muted);
    font-size: var(--fs-sm);
    font-weight: 600;
    text-align: start;
    font-variant-numeric: tabular-nums;
    border-inline-start: 3px solid transparent;
  }
  tr[data-focused] .key-cell {
    border-inline-start-color: var(--signal);
    color: var(--ink);
  }
  tr[aria-selected="true"] > * {
    background: var(--selection);
  }
  tr[data-key]:hover > * {
    background: var(--active);
  }
  td[data-active] {
    outline: 2px solid var(--signal);
    outline-offset: -2px;
  }
  .band-row td {
    height: auto;
    padding: 0;
    border-bottom: 0;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--s-3);
  }
  .editor {
    display: flex;
    align-items: center;
    height: 100%;
  }
  .editor :global(.field) {
    width: 100%;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { registerShortcut } from '../../lib/keymap';
  import { api } from '../../lib/api';
  import type { FieldRef, FixedField, Issue, IssuePatch, PropertyKind, SavedView, Value } from '../../lib/bindings';
  import { currentViewId, issues, propertyDefs, refreshAll, statuses, views } from '../../lib/stores.svelte';

  let { view }: { view: SavedView } = $props();

  type EditKind = PropertyKind | 'Status' | 'Priority';
  // Fixed fields a table can show and edit. Others are left out of the column picker and skipped in views.
  const FIXED: Partial<Record<FixedField, { label: string; kind: EditKind }>> = {
    Title: { label: 'Title', kind: 'Text' },
    Status: { label: 'Status', kind: 'Status' },
    Priority: { label: 'Priority', kind: 'Priority' },
    StartDate: { label: 'Start', kind: 'Date' },
    DueDate: { label: 'Due', kind: 'Date' },
    Estimate: { label: 'Estimate', kind: 'Number' },
  };
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
  const cols = $derived(
    def.columns
      .map((ref, index) => ({ ref, index, id: colId(ref), kind: kindOf(ref) }))
      .filter((c): c is typeof c & { kind: EditKind } => c.kind !== null),
  );
  const label = (ref: FieldRef) =>
    ref.Property !== undefined ? (propertyDefs.find((d) => d.key === ref.Property)?.name ?? '') : FIXED[ref.Fixed!]!.label;
  const addable = $derived([
    ...Object.entries(FIXED)
      .filter(([f]) => !def.columns.some((c) => c.Fixed === f))
      .map(([f, m]) => ({ id: `fixed:${f}`, name: m!.label })),
    ...propertyDefs
      .filter((d) => !def.columns.some((c) => c.Property === d.key))
      .map((d) => ({ id: `prop:${d.key}`, name: d.name })),
  ]);

  let active = $state({ row: 0, col: 0 });
  let editing = $state<{ row: number; col: number } | null>(null);
  let draft = $state('');
  let error = $state<string | null>(null);

  onMount(() => {
    currentViewId.value = view.id;
    void refreshAll();
  });

  const raw = (i: Issue, ref: FieldRef): Value => {
    if (ref.Property !== undefined) return i.properties[ref.Property] ?? null;
    switch (ref.Fixed) {
      case 'Title': return i.title;
      case 'Status': return i.status_id;
      case 'Priority': return i.priority;
      case 'StartDate': return i.start_date;
      case 'DueDate': return i.due?.Date ?? i.due?.DateTime?.utc.slice(0, 10) ?? null;
      case 'Estimate': return i.estimate_minutes;
      default: return null;
    }
  };
  const optionsOf = (kind: EditKind, ref: FieldRef) =>
    kind === 'Status'
      ? statuses.map((s) => ({ value: String(s.id), label: s.name }))
      : ref.Property !== undefined
        ? (propertyDefs.find((d) => d.key === ref.Property)?.options ?? []).map((o) => ({ value: o.value, label: o.value }))
        : [];
  const show = (v: Value, kind: EditKind): string => {
    if (v === null || v === '') return '';
    if (kind === 'Status') return statuses.find((s) => s.id === v)?.name ?? String(v);
    if (typeof v === 'boolean') return v ? 'Yes' : 'No';
    if (Array.isArray(v)) return v.join(', ');
    return typeof v === 'object' ? JSON.stringify(v) : String(v);
  };
  const draftOf = (v: Value, kind: EditKind) => (v === null ? '' : kind === 'Status' ? String(v) : Array.isArray(v) ? v.join(', ') : String(v));

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

  function startEdit() {
    const issue = issues[active.row];
    const col = cols[active.col];
    if (!issue || !col) return;
    if (col.kind === 'Checkbox') return void save(issue, col, raw(issue, col.ref) !== true);
    editing = { row: active.row, col: active.col };
    draft = draftOf(raw(issue, col.ref), col.kind);
    error = null;
  }

  function jump(delta: number) {
    if (!editing) active = { ...active, row: Math.min(Math.max(active.row + delta, 0), Math.max(issues.length - 1, 0)) };
  }

  // s and p edit the active row's Status or Priority cell when the view shows that column.
  function editField(field: FixedField) {
    const col = cols.findIndex((c) => c.ref.Fixed === field);
    if (editing || col < 0) return;
    active = { ...active, col };
    startEdit();
  }

  onMount(() => {
    const offs = [
      registerShortcut('j', 'table.next', () => jump(1)),
      registerShortcut('k', 'table.previous', () => jump(-1)),
      registerShortcut('enter', 'table.open', () => !editing && startEdit()),
      registerShortcut('s', 'table.status', () => editField('Status')),
      registerShortcut('p', 'table.priority', () => editField('Priority')),
    ];
    return () => offs.forEach((off) => off());
  });

  async function commit() {
    const e = editing!;
    const issue = issues[e.row];
    const col = cols[e.col];
    const parsed = parse(draft, col);
    if (!parsed.ok) {
      error = parsed.message;
      return;
    }
    editing = null;
    await save(issue, col, parsed.value);
  }

  function onkeydown(e: KeyboardEvent) {
    if (editing) {
      if (e.key === 'Enter') {
        e.preventDefault();
        void commit();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        editing = null;
        error = null;
      }
      return;
    }
    const clamp = (n: number, max: number) => Math.min(Math.max(n, 0), Math.max(max - 1, 0));
    const moves: Record<string, () => void> = {
      ArrowUp: () => (active = { ...active, row: clamp(active.row - 1, issues.length) }),
      ArrowDown: () => (active = { ...active, row: clamp(active.row + 1, issues.length) }),
      ArrowLeft: () => (active = { ...active, col: clamp(active.col - 1, cols.length) }),
      ArrowRight: () => (active = { ...active, col: clamp(active.col + 1, cols.length) }),
    };
    if (moves[e.key]) {
      e.preventDefault();
      moves[e.key]();
    }
  }

  async function saveColumns(columns: FieldRef[]) {
    try {
      await api.views.save(view.id, live.name, { ...def, columns });
      await refreshAll();
      error = null;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  function addColumn(e: Event) {
    const select = e.currentTarget as HTMLSelectElement;
    const id = select.value;
    if (!id) return;
    const ref: FieldRef = id.startsWith('prop:') ? { Property: id.slice(5) } : { Fixed: id.slice(6) as FixedField };
    void saveColumns([...def.columns, ref]);
  }

  function moveColumn(index: number, delta: number) {
    const columns = [...def.columns];
    const target = index + delta;
    if (target < 0 || target >= columns.length) return;
    [columns[index], columns[target]] = [columns[target], columns[index]];
    void saveColumns(columns);
  }

  const removeColumn = (index: number) => void saveColumns(def.columns.filter((_, i) => i !== index));
</script>

<section class="table-view">
  <header class="toolbar">
    <h1>{live.name}</h1>
    <label>
      Add column
      <select data-testid="add-column" onchange={addColumn}>
        <option value="">+</option>
        {#each addable as opt (opt.id)}
          <option value={opt.id}>{opt.name}</option>
        {/each}
      </select>
    </label>
  </header>

  {#if error}
    <p role="alert" class="error">{error}</p>
  {/if}

  <table role="grid" tabindex="0" {onkeydown}>
    <thead>
      <tr>
        <th scope="col">Key</th>
        {#each cols as col (col.id)}
          <th scope="col" data-col={col.id}>
            <span>{label(col.ref)}</span>
            <button type="button" aria-label="Move {label(col.ref)} left" onclick={() => moveColumn(col.index, -1)}>&lsaquo;</button>
            <button type="button" aria-label="Move {label(col.ref)} right" onclick={() => moveColumn(col.index, 1)}>&rsaquo;</button>
            <button type="button" aria-label="Remove column {label(col.ref)}" onclick={() => removeColumn(col.index)}>&times;</button>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each issues as issue, r (issue.id)}
        <tr data-key={issue.key}>
          <th scope="row">{issue.key}</th>
          {#each cols as col, c (col.id)}
            {@const isEditing = editing !== null && editing.row === r && editing.col === c}
            <td
              role="gridcell"
              data-col={col.id}
              data-active={r === active.row && c === active.col ? 'true' : undefined}
              onclick={() => (active = { row: r, col: c })}
            >
              {#if isEditing}
                {#if col.kind === 'Status' || col.kind === 'Select' || col.kind === 'MultiSelect'}
                  <select bind:value={draft} autofocus>
                    {#if col.kind !== 'Status'}<option value=""></option>{/if}
                    {#each optionsOf(col.kind, col.ref) as opt (opt.value)}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                {:else if col.kind === 'Date'}
                  <input type="date" bind:value={draft} autofocus />
                {:else}
                  <input type="text" bind:value={draft} autofocus inputmode={col.kind === 'Number' ? 'decimal' : undefined} />
                {/if}
              {:else}
                {show(raw(issue, col.ref), col.kind)}
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</section>

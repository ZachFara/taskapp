<script lang="ts" module>
  import type { FieldRef, Filter, FixedField, Layout, Op, PropertyKind, Value } from '../lib/bindings';

  export type Node =
    | { kind: 'group'; op: 'And' | 'Or'; not: boolean; items: Node[] }
    | { kind: 'cond'; field: FieldRef; op: Op; value: Value };

  const text: Op[] = ['Eq', 'Neq', 'Contains', 'IsEmpty', 'IsNotEmpty'];
  const operators: Record<PropertyKind, Op[]> = {
    Text: text,
    Url: text,
    NoteLink: text,
    Number: ['Eq', 'Neq', 'Lt', 'Lte', 'Gt', 'Gte', 'IsEmpty', 'IsNotEmpty'],
    Date: ['Eq', 'Neq', 'Lt', 'Lte', 'Gt', 'Gte', 'IsEmpty', 'IsNotEmpty'],
    Select: ['Eq', 'Neq', 'In', 'NotIn', 'IsEmpty', 'IsNotEmpty'],
    MultiSelect: ['Contains', 'In', 'IsEmpty', 'IsNotEmpty'],
    Checkbox: ['Eq'],
  };
  export const opsFor = (kind: PropertyKind) => operators[kind];

  const fixedKinds: Record<FixedField, PropertyKind> = {
    Title: 'Text', Status: 'Select', StatusCategory: 'Select', Priority: 'Number', Project: 'Select',
    Milestone: 'Number', Cycle: 'Select', Label: 'MultiSelect', Parent: 'Number', StartDate: 'Date',
    DueDate: 'Date', Estimate: 'Number', CreatedAt: 'Date', UpdatedAt: 'Date', CompletedAt: 'Date', Archived: 'Checkbox',
  };
  export const fixedFields = Object.keys(fixedKinds) as FixedField[];
  export const fixedKind = (f: FixedField) => fixedKinds[f];

  const noValue = (op: Op) => op === 'IsEmpty' || op === 'IsNotEmpty';
  const multi = (op: Op) => op === 'In' || op === 'NotIn';

  /** An empty group (or one that only holds empty groups) is dropped rather than saved as a match-nothing/everything node. */
  export function toFilter(n: Node): Filter | null {
    if (n.kind === 'cond') return { Cond: { field: n.field, op: n.op, value: noValue(n.op) ? null : n.value } };
    const parts = n.items.map(toFilter).filter((f): f is Filter => f !== null);
    if (!parts.length) return null;
    const g: Filter = n.op === 'And' ? { And: parts } : { Or: parts };
    return n.not ? { Not: g } : g;
  }

  export function fromFilter(f: Filter | null): Node {
    const root = (op: 'And' | 'Or', not: boolean, items: Node[]): Node => ({ kind: 'group', op, not, items });
    const node = (x: Filter): Node =>
      x.Cond
        ? { kind: 'cond', ...x.Cond }
        : x.Not
          ? root('And', true, [node(x.Not)])
          : root(x.Or ? 'Or' : 'And', false, (x.Or ?? x.And ?? []).map(node));
    if (!f) return root('And', false, []);
    const n = node(f);
    return n.kind === 'group' ? n : root('And', false, [n]);
  }

  export { multi, noValue };
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { api } from '../lib/api';
  import type { Cycle, Label, Project, PropertyDef, SavedView, Sort, Status, StatusCategory, ViewDef } from '../lib/bindings';

  let { view = null, onsaved }: { view?: SavedView | null; onsaved?: (v: SavedView) => void } = $props();

  // The form edits a copy; a different view means a fresh mount.
  const start = untrack(() => view);
  const d = start?.def;
  let name = $state(start?.name ?? '');
  let root = $state<Node>(fromFilter(d?.filter ?? null));
  let sorts = $state<Sort[]>(structuredClone(d?.sorts ?? []));
  let groupBy = $state<FieldRef | null>(d?.group_by ?? null);
  let layout = $state<Layout>(d?.layout ?? 'List');
  let columns = $state<FieldRef[]>(structuredClone(d?.columns ?? []));
  let includeArchived = $state(d?.include_archived ?? false);
  let error = $state('');

  let statuses = $state<Status[]>([]);
  let projects = $state<Project[]>([]);
  let defs = $state<PropertyDef[]>([]);
  let cycles = $state<Cycle[]>([]);
  let labels = $state<Label[]>([]);
  onMount(async () => {
    [statuses, projects, defs, cycles, labels] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.cycles.list(), api.labels.list(),
    ]);
  });

  const layouts: Layout[] = ['List', 'Table', 'Board', 'Gantt', 'Calendar'];
  const categories: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];
  const fieldKey = (f: FieldRef) => (f.Property !== undefined ? `P:${f.Property}` : `F:${f.Fixed}`);
  const parseField = (k: string): FieldRef => (k.startsWith('P:') ? { Property: k.slice(2) } : { Fixed: k.slice(2) as FixedField });
  const fields = $derived([
    ...fixedFields.map((f) => ({ key: `F:${f}`, label: f })),
    ...defs.map((p) => ({ key: `P:${p.key}`, label: p.name })),
  ]);
  const kindOf = (f: FieldRef): PropertyKind =>
    f.Property !== undefined ? (defs.find((p) => p.key === f.Property)?.kind ?? 'Text') : fixedKind(f.Fixed!);
  const choices = (f: FieldRef): { value: Value; label: string }[] => {
    if (f.Property !== undefined) return (defs.find((p) => p.key === f.Property)?.options ?? []).map((o) => ({ value: o.value, label: o.value }));
    const named = (rows: { id: number; name: string }[]) => rows.map((r) => ({ value: r.id, label: r.name }));
    switch (f.Fixed) {
      case 'Status': return named(statuses);
      case 'Project': return named(projects);
      case 'Cycle': return named(cycles.map((c) => ({ id: c.id, name: c.name ?? `${c.start_date} to ${c.end_date}` })));
      case 'Label': return named(labels);
      case 'StatusCategory': return categories.map((c) => ({ value: c, label: c }));
      default: return [];
    }
  };

  const isOptions = (k: PropertyKind) => k === 'Select' || k === 'MultiSelect';
  const blank = (k: PropertyKind, op: Op): Value => (multi(op) ? [] : k === 'Checkbox' ? true : null);
  function setField(c: Extract<Node, { kind: 'cond' }>, key: string) {
    c.field = parseField(key);
    const k = kindOf(c.field);
    c.op = opsFor(k)[0];
    c.value = blank(k, c.op);
  }
  function setOp(c: Extract<Node, { kind: 'cond' }>, op: Op) {
    const was = multi(c.op);
    c.op = op;
    if (was !== multi(op)) c.value = blank(kindOf(c.field), op);
  }
  const pick = (c: Extract<Node, { kind: 'cond' }>, raw: string[]) => {
    const found = raw.map((r) => choices(c.field).find((o) => String(o.value) === r)!.value);
    c.value = multi(c.op) ? found : (found[0] ?? null);
  };
  const addCond = (g: Extract<Node, { kind: 'group' }>) =>
    g.items.push({ kind: 'cond', field: { Fixed: 'Title' }, op: 'Contains', value: '' });
  const addGroup = (g: Extract<Node, { kind: 'group' }>) => g.items.push({ kind: 'group', op: 'And', not: false, items: [] });
  const remove = (g: Extract<Node, { kind: 'group' }>, n: Node) => g.items.splice(g.items.indexOf(n), 1);

  async function save(asNew: boolean) {
    error = '';
    if (!name.trim()) return void (error = 'Name is required');
    const def: ViewDef = {
      filter: toFilter(root), sorts: $state.snapshot(sorts), group_by: $state.snapshot(groupBy), layout,
      columns: $state.snapshot(columns), include_archived: includeArchived,
    };
    try {
      onsaved?.(await api.views.save(asNew ? null : (view?.id ?? null), name.trim(), def));
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
  const toggleColumn = (f: FieldRef, on: boolean) =>
    (columns = on ? [...columns, f] : columns.filter((c) => fieldKey(c) !== fieldKey(f)));
</script>

{#snippet fieldSelect(label: string, value: FieldRef | null, set: (f: FieldRef) => void)}
  <select aria-label={label} value={value ? fieldKey(value) : ''} onchange={(e) => set(parseField(e.currentTarget.value))}>
    {#each fields as f (f.key)}<option value={f.key}>{f.label}</option>{/each}
  </select>
{/snippet}

{#snippet group(g: Extract<Node, { kind: 'group' }>, nested: boolean)}
  <div class="group" role="group" aria-label="Filter group">
    <select aria-label="Match" bind:value={g.op}><option value="And">All of</option><option value="Or">Any of</option></select>
    <label><input type="checkbox" bind:checked={g.not} /> Not</label>
    {#each g.items as n (n)}
      <div class="row">
        {#if n.kind === 'group'}
          {@render group(n, true)}
        {:else}
          {@const c = n}
          {@const kind = kindOf(c.field)}
          <select aria-label="Field" value={fieldKey(c.field)} onchange={(e) => setField(c, e.currentTarget.value)}>
            {#each fields as f (f.key)}<option value={f.key}>{f.label}</option>{/each}
          </select>
          <select aria-label="Operator" value={c.op} onchange={(e) => setOp(c, e.currentTarget.value as Op)}>
            {#each opsFor(kind) as o (o)}<option value={o}>{o}</option>{/each}
          </select>
          {#if noValue(c.op)}
            <span></span>
          {:else if isOptions(kind) && choices(c.field).length}
            {@const sel = ([] as Value[]).concat(c.value ?? []).map(String)}
            <select aria-label="Value" multiple={multi(c.op)} onchange={(e) => pick(c, [...e.currentTarget.selectedOptions].map((o) => o.value))}>
              {#if !multi(c.op)}<option value="" selected={!sel.length}></option>{/if}
              {#each choices(c.field) as o (o.value)}<option value={String(o.value)} selected={sel.includes(String(o.value))}>{o.label}</option>{/each}
            </select>
          {:else if kind === 'Checkbox'}
            <input aria-label="Value" type="checkbox" checked={c.value === true} onchange={(e) => (c.value = e.currentTarget.checked)} />
          {:else if kind === 'Number'}
            <input aria-label="Value" type="number" value={c.value} oninput={(e) => (c.value = e.currentTarget.value === '' ? null : e.currentTarget.valueAsNumber)} />
          {:else}
            <input aria-label="Value" type={kind === 'Date' ? 'date' : 'text'} value={c.value ?? ''} oninput={(e) => (c.value = e.currentTarget.value)} />
          {/if}
        {/if}
        <button type="button" aria-label="Remove" onclick={() => remove(g, n)}>x</button>
      </div>
    {/each}
    <button type="button" onclick={() => addCond(g)}>Add condition</button>
    <button type="button" onclick={() => addGroup(g)}>Add group</button>
  </div>
{/snippet}

<form class="editor" onsubmit={(e) => e.preventDefault()}>
  <label>Name <input aria-label="Name" bind:value={name} /></label>
  <fieldset><legend>Filter</legend>{#if root.kind === 'group'}{@render group(root, false)}{/if}</fieldset>
  <fieldset>
    <legend>Sort</legend>
    {#each sorts as s, i (i)}
      <div class="row">
        {@render fieldSelect('Sort field', s.field, (f) => (s.field = f))}
        <label><input type="checkbox" bind:checked={s.descending} /> Descending</label>
        <button type="button" aria-label="Remove sort" onclick={() => sorts.splice(i, 1)}>x</button>
      </div>
    {/each}
    <button type="button" onclick={() => sorts.push({ field: { Fixed: 'Title' }, descending: false })}>Add sort</button>
  </fieldset>
  <label>Group by
    <select aria-label="Group by" value={groupBy ? fieldKey(groupBy) : ''} onchange={(e) => (groupBy = e.currentTarget.value ? parseField(e.currentTarget.value) : null)}>
      <option value="">None</option>
      {#each fields as f (f.key)}<option value={f.key}>{f.label}</option>{/each}
    </select>
  </label>
  <label>Layout
    <select aria-label="Layout" bind:value={layout}>{#each layouts as l (l)}<option value={l}>{l}</option>{/each}</select>
  </label>
  <fieldset>
    <legend>Columns</legend>
    {#each fields as f (f.key)}
      <label><input type="checkbox" checked={columns.some((c) => fieldKey(c) === f.key)} onchange={(e) => toggleColumn(parseField(f.key), e.currentTarget.checked)} /> {f.label}</label>
    {/each}
  </fieldset>
  <label><input type="checkbox" bind:checked={includeArchived} /> Include archived</label>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  <div class="row">
    <button type="button" onclick={() => save(false)}>{view ? 'Save' : 'Save view'}</button>
    {#if view}<button type="button" onclick={() => save(true)}>Save as new</button>{/if}
  </div>
</form>

<style>
  .editor { display: flex; flex-direction: column; gap: 8px; font-size: 13px; }
  .group { border-left: 2px solid #ccc; padding-left: 8px; display: flex; flex-direction: column; gap: 4px; }
  .row { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .error { color: #c0392b; }
  fieldset { border: 1px solid #ddd; }
</style>

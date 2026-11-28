<script lang="ts">
  // The filter, group, sort and display controls of a view. It keeps the chips it shows and reports every
  // change as a partial ViewDef; the page decides when to save.
  import { onMount, untrack, type Snippet } from 'svelte';
  import { api } from '../lib/api';
  import type { FieldRef, FixedField, ViewDef, ViewEntity } from '../lib/bindings';
  import Button from '../ui/Button.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import Popover from '../ui/Popover.svelte';
  import OptionList from './detail/OptionList.svelte';
  import Select from '../ui/Select.svelte';
  import Switch from '../ui/Switch.svelte';
  import Tooltip from '../ui/Tooltip.svelte';
  import { shortcutFor } from '../lib/commands/registry';
  import DisplayPanel from './display/DisplayPanel.svelte';
  import FilterChip from './filters/FilterChip.svelte';
  import { fieldKey, makeLookup, parseField, type Data } from './filters/lookup';
  import { fixedOn } from '../lib/fields.svelte';
  import { buildFilter, newChip, splitFilter, type Chip } from './filters/model';

  let { def, onchange, clearSignal = 0, entity = 'issue' }: { def: ViewDef; onchange: (patch: Partial<ViewDef>) => void; clearSignal?: number; entity?: ViewEntity } = $props();
  const project = untrack(() => entity === 'project');
  // The issue-field switches in Settings do not apply to projects.
  const on = (f: FixedField | undefined) => project || fixedOn(f);

  const start = untrack(() => splitFilter(def.filter));
  let chips = $state<Chip[]>(start.chips);
  let advanced = $state(start.advanced);
  let data = $state<Data>({ statuses: [], projects: [], defs: [], cycles: [], labels: [], relationKinds: [], projectStatuses: [] });
  let openId = $state<number | null>(null);
  let groupOpen = $state(false);
  let sortOpen = $state(false);
  let displayOpen = $state(false);
  let addOpen = $state(false);
  // The command menu runs the same open handlers the buttons do.
  onMount(() => {
    const on: [string, () => void][] = [
      ['view.group.open', () => (groupOpen = true)], ['view.sort.open', () => (sortOpen = true)],
      ['view.display.open', () => (displayOpen = true)], ['view.filter.add', () => (addOpen = true)],
    ];
    for (const [n, f] of on) addEventListener(`command:${n}`, f);
    return () => on.forEach(([n, f]) => removeEventListener(`command:${n}`, f));
  });
  onMount(async () => {
    const [statuses, projects, defs, cycles, labels, relationKinds, projectStatuses] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.cycles.list(), api.labels.list(), api.relation.kindsList(), api.project.statusesList(),
    ]);
    data = { statuses, projects, defs: defs.filter((d) => (project ? d.applies_to !== 'Issues' : d.applies_to !== 'Projects')), cycles, labels, relationKinds, projectStatuses };
  });
  const lookup = $derived(makeLookup(data, entity));

  // "Clear filters" from the page empties the chips too.
  let seen = untrack(() => clearSignal);
  $effect(() => {
    if (clearSignal === seen) return;
    seen = clearSignal;
    chips = [];
    advanced = null;
  });

  const commit = () => onchange({ filter: buildFilter($state.snapshot(advanced), $state.snapshot(chips)) });
  // A project view's own field lists: the fields of a project, not of an issue.
  const quick: FixedField[] = project
    ? ['Status', 'StatusCategory', 'Priority', 'Label', 'StartDate', 'TargetDate', 'CreatedAt', 'UpdatedAt', 'Progress', 'IssueCount']
    : ['Status', 'StatusCategory', 'Project', 'ProjectStatus', 'Label', 'Priority', 'DueDate', 'HiddenUntil', 'RelationKind'];
  const addable = $derived<{ field: FieldRef; name: string }[]>([
    ...quick.filter(on).map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
    ...data.defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name })),
  ]);
  function add(field: FieldRef) {
    const kind = lookup.kind(field);
    const picks = lookup.choices(field).length > 0;
    const c = picks ? newChip(field, 'In', []) : newChip(field, kind === 'Text' ? 'Contains' : kind === 'Checkbox' ? 'Eq' : kind === 'Date' ? 'Lt' : 'Eq', kind === 'Checkbox' ? true : null);
    chips.push(c);
    openId = c.id;
    commit();
  }
  const edit = (c: Chip) => {
    chips[chips.findIndex((x) => x.id === c.id)] = c;
    commit();
  };
  const remove = (c: Chip) => {
    chips = chips.filter((x) => x.id !== c.id);
    commit();
  };

  const plain: FixedField[] = project ? ['Title', 'Status', 'StatusCategory', 'Priority', 'StartDate', 'TargetDate', 'CreatedAt', 'UpdatedAt', 'Progress', 'IssueCount'] : ['Status', 'StatusCategory', 'Priority', 'Project', 'Cycle', 'DueDate', 'Points', 'Title'];
  const allFields = $derived([
    ...plain.filter(on).map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
    ...data.defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name })),
  ]);
  const fieldItems = $derived(allFields.map((f) => ({ value: fieldKey(f.field), label: f.name })));
  // Only these can group a project view: a status, the priority, or a select property.
  const groupItems = $derived(project ? fieldItems.filter((f) => ['F:Status', 'F:StatusCategory', 'F:Priority'].includes(f.value) || data.defs.some((d) => d.kind === 'Select' && f.value === `P:${d.key}`)) : fieldItems);
  const sort = $derived(def.sorts[0] ?? null);
  const sortItems = $derived(sort && !fieldItems.some((f) => f.value === fieldKey(sort.field)) ? [...fieldItems, { value: fieldKey(sort.field), label: lookup.name(sort.field) }] : fieldItems);
  const setGroup = (k: string) => onchange({ group_by: k ? parseField(k) : null });
  const setSort = (k: string) => onchange({ sorts: k ? [{ field: parseField(k), descending: sort?.descending ?? false }] : [] });
  const setDesc = (descending: boolean) => sort && onchange({ sorts: [{ ...sort, descending }] });
</script>

{#snippet tip(label: string, id: string, inner: Snippet)}
  <Tooltip {label} shortcut={shortcutFor(id)}>
    {#snippet children(p)}<span {...p} class="trig">{@render inner()}</span>{/snippet}
  </Tooltip>
{/snippet}
{#snippet addBtn()}<Button icon="filter">Add filter</Button>{/snippet}
{#snippet groupBtn()}<Button icon="group">Group{def.group_by && on(def.group_by.Fixed) ? `: ${lookup.name(def.group_by)}` : ''}</Button>{/snippet}
{#snippet sortBtn()}<Button icon="sort">Sort{sort ? `: ${lookup.name(sort.field)}` : ''}</Button>{/snippet}
{#snippet displayBtn()}<Button icon="eye">Display</Button>{/snippet}

<div class="bar">
  <div class="chips" role="group" aria-label="Filters">
    {#each chips as c (c.id)}
      <FilterChip chip={c} {lookup} open={openId === c.id} onchange={edit} onremove={() => remove(c)} />
    {/each}
    {#if advanced}
      <span class="adv"><span>Advanced filter</span>
        <IconButton icon="close" label="Remove advanced filter" onclick={() => { advanced = null; commit(); }} />
      </span>
    {/if}
    <Menu bind:open={addOpen}>
      {#snippet trigger(props)}<span {...props} class="trig">{@render tip('Add filter', 'view.filter.add', addBtn)}</span>{/snippet}
      {#each addable as a (fieldKey(a.field))}<MenuItem label={a.name} onSelect={() => add(a.field)} />{/each}
    </Menu>
  </div>
  <div class="tools">
    {#if def.layout !== 'Calendar'}
    <Popover bind:open={groupOpen}>
      {#snippet trigger(props)}<span {...props} class="trig">{@render tip('Group', 'view.group.open', groupBtn)}</span>{/snippet}
      <OptionList label="Group by" none="No grouping" value={def.group_by ? fieldKey(def.group_by) : null} options={groupItems.map((f) => ({ id: f.value, name: f.label }))} onpick={(id) => { setGroup(id === null ? '' : String(id)); groupOpen = false; }} />
    </Popover>
    {/if}
    <Popover bind:open={sortOpen}>
      {#snippet trigger(props)}<span {...props} class="trig">{@render tip('Sort', 'view.sort.open', sortBtn)}</span>{/snippet}
      <div class="stack">
        <Select label="Sort by" placeholder="Default order" items={[{ value: '', label: 'Default order' }, ...sortItems]} bind:value={() => (sort ? fieldKey(sort.field) : ''), setSort} />
        {#if sort}<Switch label="Descending" bind:checked={() => sort.descending, setDesc} />{/if}
      </div>
    </Popover>
    <Popover bind:open={displayOpen}>
      {#snippet trigger(props)}<span {...props} class="trig">{@render tip('Display', 'view.display.open', displayBtn)}</span>{/snippet}
      <DisplayPanel {def} {onchange} {entity} />
    </Popover>
  </div>
</div>

<style>
  .bar { display: flex; align-items: center; gap: var(--s-6); width: 100%; min-width: 0; }
  .chips { display: flex; align-items: center; gap: var(--s-3); flex: 1; min-width: 0; overflow-x: auto; }
  .tools { display: flex; gap: var(--s-3); flex: none; }
  .trig { display: inline-flex; }
  .stack { display: grid; gap: var(--s-4); }
  .adv {
    display: inline-flex; align-items: center; gap: var(--s-3); height: var(--row-h-compact); padding: 0 var(--s-4);
    border: 1px dashed var(--control); border-radius: var(--r-ctl); font-size: var(--fs-sm); color: var(--muted);
  }
</style>

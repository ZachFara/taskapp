<script lang="ts">
  // The filter, group, sort and display controls of a view. It keeps the chips it shows and reports every
  // change as a partial ViewDef; the page decides when to save.
  import { onMount, untrack } from 'svelte';
  import { api } from '../lib/api';
  import type { FieldRef, FixedField, ViewDef } from '../lib/bindings';
  import Button from '../ui/Button.svelte';
  import Checkbox from '../ui/Checkbox.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import Popover from '../ui/Popover.svelte';
  import Select from '../ui/Select.svelte';
  import Switch from '../ui/Switch.svelte';
  import FilterChip from './filters/FilterChip.svelte';
  import { fieldKey, makeLookup, parseField, type Data } from './filters/lookup';
  import { buildFilter, newChip, splitFilter, type Chip } from './filters/model';

  let { def, onchange, clearSignal = 0 }: { def: ViewDef; onchange: (patch: Partial<ViewDef>) => void; clearSignal?: number } = $props();

  const start = untrack(() => splitFilter(def.filter));
  let chips = $state<Chip[]>(start.chips);
  let advanced = $state(start.advanced);
  let data = $state<Data>({ statuses: [], projects: [], defs: [], cycles: [], labels: [] });
  let openId = $state<number | null>(null);
  onMount(async () => {
    const [statuses, projects, defs, cycles, labels] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.cycles.list(), api.labels.list(),
    ]);
    data = { statuses, projects, defs, cycles, labels };
  });
  const lookup = $derived(makeLookup(data));

  // "Clear filters" from the page empties the chips too.
  let seen = untrack(() => clearSignal);
  $effect(() => {
    if (clearSignal === seen) return;
    seen = clearSignal;
    chips = [];
    advanced = null;
  });

  const commit = () => onchange({ filter: buildFilter($state.snapshot(advanced), $state.snapshot(chips)) });
  const quick: FixedField[] = ['Status', 'StatusCategory', 'Project', 'Label', 'Priority', 'DueDate'];
  const addable = $derived<{ field: FieldRef; name: string }[]>([
    ...quick.map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
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

  const allFields = $derived([
    ...(['Status', 'StatusCategory', 'Priority', 'Project', 'Cycle', 'DueDate', 'Title'] as FixedField[]).map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
    ...data.defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name })),
  ]);
  const fieldItems = $derived(allFields.map((f) => ({ value: fieldKey(f.field), label: f.name })));
  const sort = $derived(def.sorts[0] ?? null);
  const setGroup = (k: string) => onchange({ group_by: k ? parseField(k) : null });
  const setSort = (k: string) => onchange({ sorts: k ? [{ field: parseField(k), descending: sort?.descending ?? false }] : [] });
  const setDesc = (descending: boolean) => sort && onchange({ sorts: [{ ...sort, descending }] });
  const hasColumn = (k: string) => def.columns.some((c) => fieldKey(c) === k);
  const setColumn = (k: string, on: boolean) =>
    onchange({ columns: on ? [...def.columns, parseField(k)] : def.columns.filter((c) => fieldKey(c) !== k) });
</script>

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
    <Menu>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="filter">Add filter</Button></span>{/snippet}
      {#each addable as a (fieldKey(a.field))}<MenuItem label={a.name} onSelect={() => add(a.field)} />{/each}
    </Menu>
  </div>
  <div class="tools">
    <Popover>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="group">Group{def.group_by ? `: ${lookup.name(def.group_by)}` : ''}</Button></span>{/snippet}
      <Select label="Group by" placeholder="No grouping" items={[{ value: '', label: 'No grouping' }, ...fieldItems]} bind:value={() => (def.group_by ? fieldKey(def.group_by) : ''), setGroup} />
    </Popover>
    <Popover>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="sort">Sort{sort ? `: ${lookup.name(sort.field)}` : ''}</Button></span>{/snippet}
      <div class="stack">
        <Select label="Sort by" placeholder="Default order" items={[{ value: '', label: 'Default order' }, ...fieldItems]} bind:value={() => (sort ? fieldKey(sort.field) : ''), setSort} />
        {#if sort}<Switch label="Descending" bind:checked={() => sort.descending, setDesc} />{/if}
      </div>
    </Popover>
    <Popover>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="eye">Display</Button></span>{/snippet}
      <div class="stack">
        <Switch label="Show archived" bind:checked={() => def.include_archived, (on) => onchange({ include_archived: on })} />
        <p class="cap">Table columns</p>
        {#each allFields as f (fieldKey(f.field))}
          <Checkbox label={f.name} bind:checked={() => hasColumn(fieldKey(f.field)), (on) => setColumn(fieldKey(f.field), on)} />
        {/each}
      </div>
    </Popover>
  </div>
</div>

<style>
  .bar { display: flex; align-items: center; gap: var(--s-6); width: 100%; min-width: 0; }
  .chips { display: flex; align-items: center; gap: var(--s-3); flex: 1; min-width: 0; overflow-x: auto; }
  .tools { display: flex; gap: var(--s-3); flex: none; }
  .trig { display: inline-flex; }
  .stack { display: grid; gap: var(--s-4); }
  .cap { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); margin-top: var(--s-3); }
  .adv {
    display: inline-flex; align-items: center; gap: var(--s-3); height: var(--row-h-compact); padding: 0 var(--s-4);
    border: 1px dashed var(--control); border-radius: var(--r-ctl); font-size: var(--fs-sm); color: var(--muted);
  }
</style>

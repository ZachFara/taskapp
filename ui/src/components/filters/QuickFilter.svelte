<script lang="ts">
  // Temporary filters on a view page: they narrow what is shown and are never stored unless "Save to view" keeps them.
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { FieldRef, FixedField, ViewEntity } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Menu from '../../ui/Menu.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import FilterChip from './FilterChip.svelte';
  import { fieldKey, makeLookup, type Data } from './lookup';
  import { newChip, type Chip } from './model';
  import { isOn } from '../../lib/features.svelte';

  let { text = $bindable(''), chips = $bindable([]), onclose, entity = 'issue' }: {
    text?: string; chips?: Chip[]; onclose: () => void; entity?: ViewEntity;
  } = $props();

  const project = entity === 'project';
  const quick: FixedField[] = project ? ['Status', 'Priority', 'Label', 'TargetDate'] : ['Status', 'Priority', 'Project', 'Label', 'LabelGroup', 'DueDate'];
  let data = $state<Data>({ statuses: [], projects: [], defs: [], cycles: [], labels: [], relationKinds: [], projectStatuses: [], labelGroups: [] });
  let openId = $state<number | null>(null);
  let addOpen = $state(false);
  const lookup = $derived(makeLookup(data, entity));
  const addable = $derived<{ field: FieldRef; name: string }[]>([
    ...quick.map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
    ...data.defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name })),
  ]);

  onMount(async () => {
    const [statuses, projects, defs, cycles, labels, projectStatuses, labelGroups] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.cycles.list(), api.labels.list(), api.project.statusesList(), api.label.groupsList(),
    ]);
    data = { statuses, projects, defs: defs.filter((d) => (project ? d.applies_to !== 'Issues' : d.applies_to !== 'Projects')), cycles, labels, relationKinds: [], projectStatuses, labelGroups };
  });

  // One click on a label of the group called Context filters to it; picking another swaps the Context chip.
  const contexts = $derived.by(() => {
    const g = data.labelGroups?.find((x) => x.name.toLowerCase() === 'context');
    return g && !project && isOn('contexts') ? data.labels.filter((l) => l.group_id === g.id) : [];
  });
  function pickContext(id: number) {
    const ids = contexts.map((l) => l.id);
    const at = chips.findIndex((c) => c.field.Fixed === 'Label' && ids.includes(([] as unknown[]).concat(c.value)[0] as number));
    const c = newChip({ Fixed: 'Label' }, 'In', [id]);
    if (at >= 0) chips[at] = c;
    else chips.push(c);
  }

  // Same chip rules as the saved filter: a pick-list field starts "any of", a text field "contains".
  function add(field: FieldRef) {
    const kind = lookup.kind(field);
    const picks = lookup.choices(field).length > 0;
    const c = picks ? newChip(field, 'In', []) : newChip(field, kind === 'Text' ? 'Contains' : kind === 'Checkbox' ? 'Eq' : kind === 'Date' ? 'Lt' : 'Eq', kind === 'Checkbox' ? true : null);
    chips.push(c);
    openId = c.id;
  }
</script>

<div class="quick" role="search" aria-label="Quick filter" onkeydown={(e) => e.key === 'Escape' && onclose()}>
  <span class="temp">Temporary</span>
  <div class="chips">
    <TextInput label="Quick filter text" placeholder="Filter by title" type="search" bind:value={text} />
    {#each chips as c (c.id)}
      <FilterChip chip={c} {lookup} open={openId === c.id}
        onchange={(n) => (chips[chips.findIndex((x) => x.id === n.id)] = n)}
        onremove={() => (chips = chips.filter((x) => x.id !== c.id))} />
    {/each}
    {#if contexts.length}
      <Menu>
        {#snippet trigger(props)}<span {...props} class="trig"><Button icon="tag">Context</Button></span>{/snippet}
        {#each contexts as l (l.id)}<MenuItem label={l.name} onSelect={() => pickContext(l.id)} />{/each}
      </Menu>
    {/if}
    <Menu bind:open={addOpen}>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="filter">Add quick filter</Button></span>{/snippet}
      {#each addable as a (fieldKey(a.field))}<MenuItem label={a.name} onSelect={() => add(a.field)} />{/each}
    </Menu>
  </div>
  <div class="tools">
    <IconButton icon="close" label="Clear and hide quick filter" shortcut="escape" onclick={onclose} />
  </div>
</div>

<style>
  .quick { display: flex; align-items: center; gap: var(--s-4); padding: var(--s-3) var(--s-6); border-top: 1px dashed var(--control); background: var(--raised); min-width: 0; }
  .temp { flex: none; font-size: var(--fs-sm); color: var(--muted); }
  .chips { display: flex; align-items: center; gap: var(--s-3); flex: 1; min-width: 0; overflow-x: auto; }
  .tools { display: flex; align-items: center; gap: var(--s-3); flex: none; }
  .trig { display: inline-flex; }
</style>

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

  let { text = $bindable(''), chips = $bindable([]), saveable, onsave, onclose, entity = 'issue' }: {
    text?: string; chips?: Chip[]; saveable: boolean; onsave: () => void; onclose: () => void; entity?: ViewEntity;
  } = $props();

  const project = entity === 'project';
  const quick: FixedField[] = project ? ['Status', 'Priority', 'Label', 'TargetDate'] : ['Status', 'Priority', 'Project', 'Label', 'DueDate'];
  let data = $state<Data>({ statuses: [], projects: [], defs: [], cycles: [], labels: [], relationKinds: [], projectStatuses: [] });
  let openId = $state<number | null>(null);
  let addOpen = $state(false);
  const lookup = $derived(makeLookup(data, entity));
  const addable = $derived<{ field: FieldRef; name: string }[]>([
    ...quick.map((f) => ({ field: { Fixed: f } as FieldRef, name: lookup.name({ Fixed: f }) })),
    ...data.defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name })),
  ]);

  onMount(async () => {
    const [statuses, projects, defs, cycles, labels, projectStatuses] = await Promise.all([
      api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.cycles.list(), api.labels.list(), api.project.statusesList(),
    ]);
    data = { statuses, projects, defs: defs.filter((d) => (project ? d.applies_to !== 'Issues' : d.applies_to !== 'Projects')), cycles, labels, relationKinds: [], projectStatuses };
  });

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
    <Menu bind:open={addOpen}>
      {#snippet trigger(props)}<span {...props} class="trig"><Button icon="filter">Add quick filter</Button></span>{/snippet}
      {#each addable as a (fieldKey(a.field))}<MenuItem label={a.name} onSelect={() => add(a.field)} />{/each}
    </Menu>
  </div>
  <div class="tools">
    {#if saveable}<Button onclick={onsave}>Save to view</Button>{/if}
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

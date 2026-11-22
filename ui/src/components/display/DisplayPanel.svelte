<script lang="ts">
  // What a view shows: every built-in field and custom property with a switch, in order, plus the
  // density and layout options. Every change is reported as a partial ViewDef.
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { FieldRef, PropertyDef, ViewDef } from '../../lib/bindings';
  import { fieldKey } from '../filters/lookup';
  import Button from '../../ui/Button.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import Switch from '../../ui/Switch.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { BUILT_IN, defaultFor, density, sameField, showEmptyGroups, showWeekends, shown, subIssues, type DisplayDef, type SubIssues } from './types';

  let { def, onchange }: { def: ViewDef; onchange: (patch: Partial<DisplayDef>) => void } = $props();

  let defs = $state<PropertyDef[]>([]);
  let query = $state('');
  let dragFrom = $state<string | null>(null);
  onMount(async () => (defs = await api.properties.listDefs()));

  const universe = $derived([...BUILT_IN, ...defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name }))]);
  const nameOf = (f: FieldRef) => universe.find((u) => sameField(u.field, f))?.name ?? '';
  const on = $derived(shown(def, defaultFor(def.layout)));
  // Displayed fields first, in their order, then the rest; the search narrows both.
  const rows = $derived(
    [...on.map((f) => ({ field: f, name: nameOf(f), on: true })), ...universe.filter((u) => !on.some((f) => sameField(f, u.field))).map((u) => ({ ...u, on: false }))].filter(
      (r) => r.name && r.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );
  const toggle = (f: FieldRef, next: boolean) => onchange({ columns: next ? [...on, f] : on.filter((x) => !sameField(x, f)) });
  function move(key: string, by: number) {
    const from = on.findIndex((f) => fieldKey(f) === key);
    const to = from + by;
    if (from < 0 || to < 0 || to >= on.length) return;
    const next = [...on];
    next.splice(to, 0, next.splice(from, 1)[0]);
    onchange({ columns: next });
  }
  const dropOn = (key: string) => {
    const from = on.findIndex((f) => fieldKey(f) === dragFrom);
    const to = on.findIndex((f) => fieldKey(f) === key);
    dragFrom = null;
    if (from >= 0 && to >= 0 && from !== to) move(fieldKey(on[from]), to - from);
  };
  const dated = $derived(def.layout === 'Gantt' || def.layout === 'Calendar');
</script>

<div class="panel" role="group" aria-label="Display options">
  <TextInput type="search" label="Search fields" placeholder="Search fields" bind:value={query} />
  <ul class="fields" aria-label="Fields">
    {#each rows as r (fieldKey(r.field))}
      <li
        class="field"
        data-field={fieldKey(r.field)}
        data-on={r.on ? '' : undefined}
        draggable={r.on}
        ondragstart={() => (dragFrom = fieldKey(r.field))}
        ondragover={(e) => r.on && e.preventDefault()}
        ondrop={() => dropOn(fieldKey(r.field))}
      >
        <span class="name">{r.name}</span>
        {#if r.on}
          <IconButton icon="arrow-up" label={`Move ${r.name} up`} onclick={() => move(fieldKey(r.field), -1)} />
          <IconButton icon="arrow-down" label={`Move ${r.name} down`} onclick={() => move(fieldKey(r.field), 1)} />
        {/if}
        <Switch label={`Show ${r.name}`} bind:checked={() => r.on, (v) => toggle(r.field, v)} />
      </li>
    {/each}
  </ul>
  <Button icon="undo" onclick={() => onchange({ columns: [] })}>Reset to default</Button>
  <div class="opts">
    <div class="opt"><span>Compact rows</span><Switch label="Compact rows" bind:checked={() => density(def) === 'compact', (v) => onchange({ density: v ? 'compact' : 'comfortable' })} /></div>
    <div class="opt"><span>Show empty groups</span><Switch label="Show empty groups" bind:checked={() => showEmptyGroups(def), (v) => onchange({ show_empty_groups: v })} /></div>
    <div class="opt"><span>Show archived</span><Switch label="Show archived" bind:checked={() => def.include_archived, (v) => onchange({ include_archived: v })} /></div>
    <div class="opt"><span>Sub-issues</span><Select label="Sub-issues" items={[{ value: 'nest', label: 'Nested' }, { value: 'flat', label: 'Flat' }, { value: 'hide', label: 'Hidden' }]} bind:value={() => subIssues(def), (v) => onchange({ show_sub_issues: v as SubIssues })} /></div>
    {#if dated}<div class="opt"><span>Show weekends</span><Switch label="Show weekends" bind:checked={() => showWeekends(def), (v) => onchange({ show_weekends: v })} /></div>{/if}
  </div>
</div>

<style>
  .panel { display: grid; gap: var(--s-5); width: var(--panel-w); }
  .fields { list-style: none; margin: 0; padding: 0; max-height: 280px; overflow-y: auto; display: grid; }
  .field { display: flex; align-items: center; gap: var(--s-3); min-height: var(--row-h-compact); color: var(--muted); }
  .field[data-on] { color: var(--ink); cursor: grab; }
  .opt { display: flex; align-items: center; justify-content: space-between; gap: var(--s-4); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .opts { display: grid; gap: var(--s-4); padding-top: var(--s-4); border-top: 1px solid var(--border); }
</style>

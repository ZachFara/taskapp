<script lang="ts">
  // What a view shows: every built-in field and custom property with a switch, in order, plus the
  // density and layout options. Every change is reported as a partial ViewDef.
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { FieldRef, PropertyDef, ViewDef, ViewEntity } from '../../lib/bindings';
  import { PROJECT_BUILT_IN, PROJECT_DEFAULT } from '../../lib/projectViews';
  import { fieldKey } from '../filters/lookup';
  import Button from '../../ui/Button.svelte';
  import Icon from '../../ui/Icon.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import Switch from '../../ui/Switch.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { fixedOn } from '../../lib/fields.svelte';
  import { BUILT_IN, KIND_ICON, defaultFor, density, sameField, showEmptyGroups, showWeekends, shown, stored, subIssues, type DisplayDef, type SubIssues } from './types';

  let { def, onchange, entity = 'issue' }: { def: ViewDef; onchange: (patch: Partial<DisplayDef>) => void; entity?: ViewEntity } = $props();
  const project = $derived(entity === 'project');
  // The issue-field switches in Settings do not apply to projects.
  const on_ = (f: FieldRef['Fixed']) => project || fixedOn(f);

  type Entry = { field: FieldRef; name: string; kind?: PropertyDef['kind'] };
  let defs = $state<PropertyDef[]>([]);
  let query = $state('');
  let dragFrom = $state<string | null>(null);
  let focused = $state<string | null>(null);
  onMount(async () => {
    defs = (await api.properties.listDefs()).filter((d) => (project ? d.applies_to !== 'Issues' : d.applies_to !== 'Projects'));
  });

  // The board draws its cards from card_fields when set; every other layout from columns.
  const card = $derived(def.layout === 'Board');
  // A field switched off in Settings is not shown but stays in the saved list, so it is back when the field is on again.
  const edit = (list: FieldRef[], reset = false): Partial<DisplayDef> => {
    const kept = reset ? list : [...list, ...stored(def, card).filter((f) => !on_(f.Fixed))];
    return card ? { card_fields: kept } : { columns: kept };
  };
  const universe = $derived<Entry[]>([...(project ? PROJECT_BUILT_IN : BUILT_IN).filter((b) => on_(b.field.Fixed)), ...defs.map((p) => ({ field: { Property: p.key } as FieldRef, name: p.name, kind: p.kind }))]);
  const on = $derived(project ? (stored(def, card).length ? stored(def, card) : PROJECT_DEFAULT[def.layout]) : shown(def, defaultFor(def.layout), card));
  const match = (e: Entry) => e.name && e.name.toLowerCase().includes(query.trim().toLowerCase());
  const entryOf = (f: FieldRef) => universe.find((u) => sameField(u.field, f));
  const byName = (a: Entry, b: Entry) => a.name.localeCompare(b.name);
  const sections = $derived.by(() => {
    const shownRows = on.flatMap((f) => entryOf(f) ?? []).filter(match);
    const off = universe.filter((u) => !on.some((f) => sameField(f, u.field)) && match(u));
    return [
      { title: 'Shown', rows: shownRows, on: true },
      // Custom properties come before the long built-in list so they are in view without scrolling.
      { title: 'Custom properties', rows: off.filter((u) => u.kind).sort(byName), on: false },
      { title: 'Hidden', rows: off.filter((u) => !u.kind).sort(byName), on: false },
    ].filter((s) => s.rows.length);
  });

  const toggle = (f: FieldRef, next: boolean) => onchange(edit(next ? [...on, f] : on.filter((x) => !sameField(x, f))));
  function move(key: string, by: number) {
    const from = on.findIndex((f) => fieldKey(f) === key);
    const to = from + by;
    if (from < 0 || to < 0 || to >= on.length) return;
    const next = [...on];
    next.splice(to, 0, next.splice(from, 1)[0]);
    onchange(edit(next));
  }
  const dropOn = (key: string) => {
    const from = on.findIndex((f) => fieldKey(f) === dragFrom);
    const to = on.findIndex((f) => fieldKey(f) === key);
    dragFrom = null;
    if (from >= 0 && to >= 0 && from !== to) move(fieldKey(on[from]), to - from);
  };
  // view.field.up / view.field.down move the field whose row has focus.
  $effect(() => {
    const step = (by: number) => () => focused && move(focused, by);
    const [up, down] = [step(-1), step(1)];
    addEventListener('command:view.field.up', up);
    addEventListener('command:view.field.down', down);
    return () => {
      removeEventListener('command:view.field.up', up);
      removeEventListener('command:view.field.down', down);
    };
  });
  const L = $derived(def.layout);
  const dated = $derived(!project && (L === 'Gantt' || L === 'Calendar'));
  const grouped = $derived(L === 'List' || L === 'Table');
</script>

<div class="panel" role="group" aria-label="Display options">
  <TextInput type="search" label="Search fields" placeholder="Search fields" bind:value={query} />
  <div class="scroll">
    {#each sections as s (s.title)}
      <section data-section={s.title}>
        <h3>{s.title}<span class="n">{s.rows.length}</span></h3>
        <ul class="fields" aria-label={s.title}>
          {#each s.rows as r (fieldKey(r.field))}
            <li
              class="field"
              data-field={fieldKey(r.field)}
              data-on={s.on ? '' : undefined}
              tabindex="-1"
              draggable={s.on}
              onfocusin={() => (focused = fieldKey(r.field))}
              ondragstart={() => (dragFrom = fieldKey(r.field))}
              ondragover={(e) => s.on && e.preventDefault()}
              ondrop={() => dropOn(fieldKey(r.field))}
            >
              {#if s.on}<span class="grip" aria-hidden="true"><Icon name="more" size={14} /></span>{/if}
              {#if r.kind}<span class="type" role="img" aria-label={r.kind}><Icon name={KIND_ICON[r.kind]} size={14} /></span>{/if}
              <span class="name">{r.name}</span>
              {#if s.on}
                <IconButton icon="arrow-up" label={`Move ${r.name} up`} onclick={() => move(fieldKey(r.field), -1)} />
                <IconButton icon="arrow-down" label={`Move ${r.name} down`} onclick={() => move(fieldKey(r.field), 1)} />
              {/if}
              <Switch label={`Show ${r.name}`} bind:checked={() => s.on, (v) => toggle(r.field, v)} />
            </li>
          {/each}
        </ul>
      </section>
    {:else}
      <p class="none">No field matches "{query}".</p>
    {/each}
  </div>
  <Button icon="undo" onclick={() => onchange(edit([], true))}>Reset to default</Button>
  <div class="opts">
    {#if L !== 'Gantt' && L !== 'Calendar'}
      <div class="opt"><span>Compact rows</span><Switch label="Compact rows" bind:checked={() => density(def) === 'compact', (v) => onchange({ density: v ? 'compact' : 'comfortable' })} /></div>
      <div class="opt"><span>Wrap</span><Switch label="Wrap" bind:checked={() => def.wrap === 'on', (v) => onchange({ wrap: v ? 'on' : 'off' })} /></div>
    {/if}
    {#if grouped}
      <div class="opt"><span>Show empty groups</span><Switch label="Show empty groups" bind:checked={() => showEmptyGroups(def), (v) => onchange({ show_empty_groups: v })} /></div>
    {/if}
    <div class="opt"><span>Show archived</span><Switch label="Show archived" bind:checked={() => def.include_archived, (v) => onchange({ include_archived: v })} /></div>
    {#if L === 'List' && !project}
      <div class="opt"><span>Sub-issues</span><Select label="Sub-issues" items={[{ value: 'nest', label: 'Nested' }, { value: 'flat', label: 'Flat' }, { value: 'hide', label: 'Hidden' }]} bind:value={() => subIssues(def), (v) => onchange({ show_sub_issues: v as SubIssues })} /></div>
    {/if}
    {#if dated}<div class="opt"><span>Show weekends</span><Switch label="Show weekends" bind:checked={() => showWeekends(def), (v) => onchange({ show_weekends: v })} /></div>{/if}
  </div>
</div>

<style>
  .panel { display: grid; gap: var(--s-5); width: var(--panel-w); }
  .scroll { display: grid; gap: var(--s-5); max-height: min(420px, 55vh); overflow-y: auto; margin-inline: calc(var(--s-3) * -1); padding-inline: var(--s-3); }
  h3 { display: flex; align-items: center; gap: var(--s-3); margin: 0 0 var(--s-2); font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); }
  .n { font-weight: 500; }
  .fields { list-style: none; margin: 0; padding: 0; display: grid; }
  .field { display: flex; align-items: center; gap: var(--s-3); min-height: var(--row-h-compact); color: var(--muted); border-radius: var(--r-ctl); }
  .field:hover { background: var(--raised); }
  .field[data-on] { color: var(--ink); }
  .grip { display: grid; place-items: center; color: var(--muted); cursor: grab; transform: rotate(90deg); }
  .type { display: grid; place-items: center; color: var(--muted); }
  .opt { display: flex; align-items: center; justify-content: space-between; gap: var(--s-4); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .none { margin: 0; color: var(--muted); }
  .opts { display: grid; gap: var(--s-4); padding-top: var(--s-4); border-top: 1px solid var(--border); }
</style>

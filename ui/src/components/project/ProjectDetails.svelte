<script lang="ts">
  import LabelPicker from '../detail/LabelPicker.svelte';
  import { api } from '../../lib/api';
  import type { Label, Project, ProjectStatus, PropertyDef, Value } from '../../lib/bindings';
  import { glyphCategory } from '../../lib/projectFields';
  import { PRIORITIES, priorityGlyph } from '../detail/format';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import OptionList from '../detail/OptionList.svelte';
  import PropertyRow from '../detail/PropertyRow.svelte';
  import PropertyRows from '../detail/PropertyRows.svelte';

  // The project's fields as a plate, like an issue's: each row opens its editor, each edit is one update.
  let { project, onsaved }: { project: Project; onsaved: (p: Project) => void } = $props();

  let statuses = $state<ProjectStatus[]>([]);
  let labels = $state<Label[]>([]);
  let defs = $state<PropertyDef[]>([]);
  let openRow = $state<string | null>(null);
  $effect(() => {
    Promise.all([api.project.statusesList(), api.labels.list(), api.properties.listDefs()]).then(([s, l, d]) => ([statuses, labels, defs] = [s, l, d]));
  });
  const status = $derived(statuses.find((s) => s.id === project.status_id));
  const visible = $derived(defs.filter((d) => d.applies_to !== 'Issues'));
  const save = async (patch: Partial<Project>) => onsaved(await api.projects.update({ ...project, ...patch }));
  const toggleLabel = (id: number, on: boolean) => save({ label_ids: on ? [...project.label_ids, id] : project.label_ids.filter((x) => x !== id) });
  const createLabel = async (name: string) => {
    const made = await api.labels.create(name, 'gray', null);
    labels = [...labels, made];
    await toggleLabel(made.id, true);
  };
  const setProp = (key: string, value: Value) => {
    const { [key]: _, ...rest } = project.properties;
    return save({ properties: value === null ? rest : { ...rest, [key]: value } });
  };
</script>

<section class="plate" aria-label="Project details">
  <PropertyRow id="status" name="Status" cmd="project.status" bind:openRow>
    {#snippet value()}{#if status}<StatusGlyph category={glyphCategory[status.category]} />{status.name}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Status" value={project.status_id} options={statuses} onpick={(id) => { close(); void save({ status_id: Number(id) }); }}>
        {#snippet lead(o)}<StatusGlyph category={glyphCategory[statuses.find((s) => s.id === o.id)!.category]} />{/snippet}
      </OptionList>
    {/snippet}
  </PropertyRow>
  <PropertyRow id="priority" name="Priority" cmd="project.priority" bind:openRow>
    {#snippet value()}<PriorityGlyph level={priorityGlyph(project.priority)} />{PRIORITIES[project.priority]}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Priority" value={project.priority} options={PRIORITIES.map((name, id) => ({ id, name }))} onpick={(id) => { close(); void save({ priority: Number(id) }); }}>
        {#snippet lead(o)}<PriorityGlyph level={priorityGlyph(Number(o.id))} />{/snippet}
      </OptionList>
    {/snippet}
  </PropertyRow>
  <PropertyRow id="labels" name="Labels" cmd="project.labels" bind:openRow>
    {#snippet value()}
      {#each labels.filter((l) => project.label_ids.includes(l.id)) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{:else}<span class="none">None</span>{/each}
    {/snippet}
    {#snippet editor()}
      <LabelPicker {labels} selected={project.label_ids} ontoggle={toggleLabel} oncreate={createLabel} />
    {/snippet}
  </PropertyRow>
  <PropertyRows defs={visible} values={project.properties} {setProp} bind:openRow />
</section>

<style>
  .plate { display: grid; align-content: start; max-width: 28rem; }
  .none { color: var(--muted); font-weight: 400; }
</style>

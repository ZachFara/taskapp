<script lang="ts">
  import { askDelete } from '../../lib/rowUi.svelte';
  import LabelPicker from './LabelPicker.svelte';
  import { api } from '../../lib/api';
  import type { Cycle, Depth, Issue, IssuePatch, Label, Milestone, Project, PropertyDef, Status, Value } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { dueFrom, dueParts, humanDate, isDate, isTime, minutes, PRIORITIES, priorityGlyph } from './format';
  import IssuePicker from './IssuePicker.svelte';
  import OptionList from './OptionList.svelte';
  import { fieldList, fieldOn } from '../../lib/fields.svelte';
  import { isOn } from '../../lib/features.svelte';
  import PointsRow from '../insights/PointsRow.svelte';
  import PropertyRow from './PropertyRow.svelte';
  import PropertyRows from './PropertyRows.svelte';

  let { issue, save, act, openRow = $bindable(null) }: {
    issue: Issue;
    save: (p: IssuePatch) => void;
    act: (p: Promise<unknown>) => void;
    openRow?: string | null;
  } = $props();

  let statuses = $state<Status[]>([]);
  let projects = $state<Project[]>([]);
  let cycles = $state<Cycle[]>([]);
  let labels = $state<Label[]>([]);
  let milestones = $state<Milestone[]>([]);
  let defs = $state<PropertyDef[]>([]);
  let parent = $state<Issue | null>(null);

  $effect(() => {
    Promise.all([api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list(), api.properties.listDefs()]).then(([s, p, c, l, d]) => {
      [statuses, projects, cycles, labels, defs] = [s, p, c, l, d];
    });
  });
  $effect(() => {
    const id = issue.project_id;
    (id === null ? Promise.resolve([]) : api.projects.listMilestones(id)).then((m) => (milestones = m));
  });
  $effect(() => {
    const id = issue.parent_id;
    (id === null ? Promise.resolve(null) : api.issues.get(id)).then((p) => (parent = p));
  });

  const status = $derived(statuses.find((s) => s.id === issue.status_id));
  const project = $derived(projects.find((p) => p.id === issue.project_id));
  const cycle = $derived(cycles.find((c) => c.id === issue.cycle_id));
  const milestone = $derived(milestones.find((m) => m.id === issue.milestone_id));
  const cycleName = (c: Cycle) => c.name ?? `Cycle ${c.number}`;
  // A definition scoped to another project does not apply to this issue.
  const visible = $derived(defs.filter((d) => d.applies_to !== 'Projects' && (d.project_id === null || d.project_id === issue.project_id)));
  const due = $derived(dueParts(issue.due));
  const num = (v: string | number | null) => (v === null ? null : Number(v));

  let draft = $state('');
  const setProp = (key: string, value: Value) => act(api.properties.setValue(issue.id, key, value));
  const toggleLabel = (id: number, on: boolean) => act(on ? api.labels.addLabel(issue.id, id) : api.labels.removeLabel(issue.id, id));
  // A label renamed or deleted elsewhere (the delete dialog, settings) shows here at once.
  $effect(() => {
    const reload = () => void api.labels.list().then((l) => (labels = l));
    addEventListener('rows:changed', reload);
    return () => removeEventListener('rows:changed', reload);
  });
  const createLabel = async (name: string) => {
    const made = await api.labels.create(name, 'gray', null);
    labels = [...labels, made];
    toggleLabel(made.id, true);
  };
</script>

{#snippet none()}<span class="none">None</span>{/snippet}

<section class="plate" aria-label="Properties">
  <PropertyRow id="status" name="Status" cmd="issue.detail.status" bind:openRow>
    {#snippet value()}{#if status}<StatusGlyph category={status.category} icon={status.icon} color={status.color} />{status.name}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Status" value={issue.status_id} options={statuses} onpick={(id) => { close(); save({ status_id: num(id) }); }}>
        {#snippet lead(o)}{@const st = statuses.find((s) => s.id === o.id)!}<StatusGlyph category={st.category} icon={st.icon} color={st.color} />{/snippet}
      </OptionList>
    {/snippet}
  </PropertyRow>

  {#snippet row_priority()}
  <PropertyRow id="priority" name="Priority" cmd="issue.detail.priority" bind:openRow>
    {#snippet value()}<PriorityGlyph level={priorityGlyph(issue.priority)} />{PRIORITIES[issue.priority]}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Priority" value={issue.priority} options={PRIORITIES.map((name, id) => ({ id, name }))} onpick={(id) => { close(); save({ priority: num(id) }); }}>
        {#snippet lead(o)}<PriorityGlyph level={priorityGlyph(Number(o.id))} />{/snippet}
      </OptionList>
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_project()}
  <PropertyRow id="project" name="Project" cmd="issue.detail.project" bind:openRow>
    {#snippet value()}{#if project}<EntityIcon icon={project.icon ?? 'icon:project'} color={project.color} />{project.name}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Project" none="No project" value={issue.project_id} options={projects} onpick={(id) => { close(); save({ project_id: num(id), milestone_id: null }); }}>
        {#snippet lead(o)}{@const p = projects.find((x) => x.id === o.id)!}<EntityIcon icon={p.icon ?? 'icon:project'} color={p.color} />{/snippet}
      </OptionList>
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_milestone()}
  <PropertyRow id="milestone" name="Milestone" cmd="issue.detail.milestone" bind:openRow>
    {#snippet value()}{#if milestone}{milestone.name}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Milestone" none="No milestone" value={issue.milestone_id} options={milestones} onpick={(id) => { close(); save({ milestone_id: num(id) }); }} />
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_cycle()}
  <PropertyRow id="cycle" name="Cycle" cmd="issue.detail.cycle" bind:openRow>
    {#snippet value()}{#if cycle}{cycleName(cycle)}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Cycle" none="No cycle" value={issue.cycle_id} options={cycles.map((c) => ({ id: c.id, name: cycleName(c) }))} onpick={(id) => { close(); save({ cycle_id: num(id) }); }} />
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_labels()}
  <PropertyRow id="labels" name="Labels" cmd="issue.detail.labels" bind:openRow>
    {#snippet value()}
      {#each labels.filter((l) => issue.label_ids.includes(l.id)) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{:else}{@render none()}{/each}
    {/snippet}
    {#snippet editor()}
      <LabelPicker {labels} selected={issue.label_ids} ontoggle={toggleLabel} oncreate={createLabel} ondelete={(l) => askDelete({ kind: 'label', id: l.id, name: l.name })} />
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_start()}
  <PropertyRow id="start" name="Start" cmd="issue.detail.start" bind:openRow>
    {#snippet value()}{humanDate(issue.start_date)}{/snippet}
    {#snippet editor()}
      <TextInput type="date" label="Start date" value={issue.start_date ?? ''} oninput={(e) => { const v = (e.currentTarget as HTMLInputElement).value; if (isDate(v)) save({ start_date: v || null }); }} />
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_due()}
  <PropertyRow id="due" name="Due" cmd="issue.detail.due" bind:openRow>
    {#snippet value()}{due.date ? `${humanDate(due.date)}${due.timed ? ` ${due.time}` : ''}` : 'None'}{/snippet}
    {#snippet editor()}
      <div class="stack">
        <TextInput type="date" label="Due date" value={due.date} oninput={(e) => { const v = (e.currentTarget as HTMLInputElement).value; if (isDate(v)) save({ due: dueFrom(v, due.timed ? due.time : null) }); }} />
        <span class="inline"><Checkbox label="Due time" bind:checked={() => due.timed, (on) => on !== due.timed && save({ due: dueFrom(due.date, on ? due.time : null) })} disabled={!due.date} /> Time of day</span>
        {#if due.timed}
          <TextInput type="time" label="Due time of day" value={due.time} onchange={(v) => isTime(v) && save({ due: dueFrom(due.date, v) })} />
        {/if}
      </div>
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_estimate()}
  <PropertyRow id="estimate" name="Estimate" cmd="issue.detail.estimate" bind:openRow>
    {#snippet value()}{#if issue.estimate_minutes !== null}{minutes(issue.estimate_minutes)}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <form class="stack" onsubmit={(e) => { e.preventDefault(); close(); save({ estimate_minutes: draft.trim() === '' ? null : Number(draft) }); }}>
        <TextInput type="number" label="Estimate (min)" value={String(issue.estimate_minutes ?? '')} oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)} />
        <Button type="submit" variant="primary">Set</Button>
      </form>
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_points()}<PointsRow {issue} {save} bind:openRow />{/snippet}

  {#snippet row_depth()}
  <PropertyRow id="depth" name="Depth" cmd="issue.depth" bind:openRow>
    {#snippet value()}{issue.depth === null ? '' : issue.depth === 'deep' ? 'Deep work' : 'Shallow work'}{#if issue.depth === null}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <OptionList label="Depth" value={issue.depth ?? 'none'} options={[{ id: 'deep', name: 'Deep work' }, { id: 'shallow', name: 'Shallow work' }, { id: 'none', name: 'No depth' }]} onpick={(id) => { close(); save({ depth: id === 'none' ? null : (id as Depth) }); }} />
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#snippet row_parent()}
  <PropertyRow id="parent" name="Parent" cmd="issue.detail.parent" bind:openRow>
    {#snippet value()}{#if parent}{parent.key} {parent.title}{:else}{@render none()}{/if}{/snippet}
    {#snippet editor(close)}
      <div class="stack">
        <IssuePicker selfId={issue.id} label="Find parent issue" pick={(p) => { close(); save({ parent_id: p.id }); }} />
        {#if parent}<Button onclick={() => { close(); save({ parent_id: null }); }}>Remove parent</Button>{/if}
      </div>
    {/snippet}
  </PropertyRow>
  {/snippet}

  {#each fieldList() as f (f.field)}
    {#if fieldOn(f.field) && (f.field !== 'cycle' || isOn('cycles'))}
      {@const rows = { priority: row_priority, project: row_project, milestone: row_milestone, cycle: row_cycle, labels: row_labels, start: row_start, due: row_due, estimate: row_estimate, points: row_points, depth: row_depth, parent: row_parent }}
      {@render rows[f.field]()}
    {/if}
  {/each}

  <PropertyRows defs={visible} values={issue.properties} {setProp} bind:openRow />
</section>

<style>
  .plate { display: grid; align-content: start; }
  .none { color: var(--muted); font-weight: 400; }
  .stack { display: grid; gap: var(--s-4); }
  .inline { display: flex; align-items: center; gap: var(--s-4); }
</style>

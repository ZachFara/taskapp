<script lang="ts">
  import { api } from '../../lib/api';
  import type { Project } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import Contour from '../../ui/Contour.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import Popover from '../../ui/Popover.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { humanDate } from '../detail/format';
  import ProgressSummary from '../insights/ProgressSummary.svelte';

  let { project, progress, renaming = $bindable(false), onsaved }: {
    project: Project;
    progress: [number, number];
    renaming?: boolean;
    onsaved: (p: Project) => void;
  } = $props();

  let name = $state('');
  let editingDesc = $state(false);
  let desc = $state('');
  $effect(() => {
    if (renaming) name = project.name;
  });

  async function saveName(e: Event) {
    e.preventDefault();
    const clean = name.trim();
    renaming = false;
    if (clean && clean !== project.name) onsaved(await api.projects.update({ ...project, name: clean }));
  }
  async function saveDescription(e: Event) {
    e.preventDefault();
    editingDesc = false;
    onsaved(await api.projects.update({ ...project, description: desc.trim() || null }));
  }
  let openDate = $state<string | null>(null);
  async function saveDate(field: 'start_date' | 'target_date', value: string) {
    onsaved(await api.projects.update({ ...project, [field]: value || null }));
  }
</script>

<div class="hero">
  <div class="art" aria-hidden="true"><Contour rings={11} size={260} /></div>
  <div class="main">
    <div class="title">
      <EntityIcon icon={project.icon ?? 'icon:project'} color={project.color} size={28} />
      {#if renaming}
        <form onsubmit={saveName}>
          <TextInput label="Project name" bind:value={name} />
          <Button variant="primary" type="submit">Save</Button>
          <Button onclick={() => (renaming = false)}>Cancel</Button>
        </form>
      {:else}
        <h2>{project.name}</h2>
      {/if}
    </div>

    {#if project.description}
      <p class="desc" data-testid="description">{project.description}</p>
    {:else if editingDesc}
      <form class="descform" onsubmit={saveDescription}>
        <TextArea label="Description" bind:value={desc} />
        <Button variant="primary" type="submit">Save</Button>
      </form>
    {:else}
      <div><Button icon="plus" onclick={() => { desc = ''; editingDesc = true; }}>Add description</Button></div>
    {/if}
  </div>

  <dl class="facts">
    <div class="progress" data-testid="progress">
      <dt>Progress</dt>
      <dd>
        <ProgressSummary projectId={project.id} {progress} />
      </dd>
    </div>
    <div>
      <dt>Start</dt>
      <dd>{@render dateField('start_date', 'Start date', 'Set start')}</dd>
    </div>
    <div>
      <dt>Target</dt>
      <dd>{@render dateField('target_date', 'Target date', 'Set target')}</dd>
    </div>
  </dl>
</div>

{#snippet dateField(field: 'start_date' | 'target_date', label: string, empty: string)}
  <Popover bind:open={() => openDate === field, (v) => (openDate = v ? field : null)}>
    {#snippet trigger(props)}
      <button {...props} type="button" class="date" class:unset={!project[field]} aria-label={label}>{humanDate(project[field], new Date(), empty)}</button>
    {/snippet}
    <TextInput label="{label} value" type="date" value={project[field] ?? ''} oninput={(e) => saveDate(field, (e.currentTarget as HTMLInputElement).value)} />
  </Popover>
{/snippet}

<style>
  .hero { position: relative; display: flex; gap: var(--s-9); align-items: flex-start; justify-content: space-between; padding: var(--s-8) var(--s-7); border-bottom: 1px solid var(--border); background: var(--band-1); overflow: hidden; }
  .art { position: absolute; right: var(--s-7); top: calc(var(--s-8) * -1); color: var(--band-3); opacity: 0.7; pointer-events: none; }
  .main { position: relative; display: grid; gap: var(--s-6); max-width: 64ch; flex: 1; }
  .title { display: flex; align-items: center; gap: var(--s-5); }
  h2 { font-size: var(--fs-display); font-weight: 800; letter-spacing: -0.03em; line-height: var(--lh-tight); }
  .desc { font-size: var(--fs-lg); line-height: var(--lh-body); }
  form { display: flex; gap: var(--s-4); align-items: center; }
  .descform { align-items: flex-start; flex-direction: column; }
  .facts { position: relative; --chart-bg: var(--band-1); display: grid; gap: var(--s-5); min-width: 280px; margin: 0; }
  dt { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--band-count); margin-bottom: var(--s-2); }
  dd { margin: 0; }
  .date { height: var(--ctl-h); padding: 0 var(--s-5); border: 1px solid var(--control); border-radius: var(--r-ctl); background: var(--sheet); color: var(--ink); font-weight: 600; }
  .date.unset { color: var(--muted); font-weight: 400; }
  .date:hover { border-color: var(--muted); }
  .progress dd { display: grid; gap: var(--s-3); }
</style>

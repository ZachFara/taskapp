<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { projects, refreshAll } from '../../lib/stores.svelte';
  import type { Project } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  let slug = $state('');
  let name = $state('');
  let look = $state<{ icon: string | null; color: string | null }>({ icon: null, color: null });
  let error = $state('');
  let doomed = $state<Project | null>(null);

  onMount(() => refreshAll());

  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  // The store keeps only the short form of a project, so a save starts from the full record.
  const save = (id: number, patch: Partial<Project>) =>
    write(async () => {
      const full = (await api.projects.list()).find((p) => p.id === id) as Project;
      await api.projects.update({ ...full, ...patch });
    });
  const add = () => write(async () => {
    await api.projects.create(slug.trim(), name.trim(), look.icon, look.color);
    slug = name = '';
    look = { icon: null, color: null };
  });
  const rename = (p: Project, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== p.name) void save(p.id, { name: next });
  };
  const remove = () => write(async () => {
    await api.projects.delete(doomed!.id);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="Each project has a short key and groups its own issues, milestones and properties." />

  {#if projects.length}
    <ul class="es-list">
      {#each projects as p (p.id)}
        <li class="es-row">
          <IconColorPicker label={`Icon and colour of ${p.name}`} value={{ icon: (p as Project).icon ?? null, color: (p as Project).color ?? null }} onchange={(v) => save(p.id, { icon: v.icon, color: v.color })} />
          <div class="es-name" onchange={(e) => rename(p as Project, e)}><TextInput label={`Name of ${p.name}`} value={p.name} /></div>
          <span class="es-meta">{p.slug}</span>
          <div class="es-tools"><IconButton icon="trash" label={`Delete ${p.name}`} onclick={() => (doomed = p as Project)} /></div>
        </li>
      {/each}
    </ul>
  {:else}
    <EmptyState title="No projects yet" hint="A project is a map of related work. Add the first one below." />
  {/if}

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <IconColorPicker label="New project icon and colour" value={look} onchange={(v) => (look = v)} />
    <div class="es-name"><TextInput label="Project name" placeholder="Name" bind:value={name} /></div>
    <div class="es-name"><TextInput label="Project slug" placeholder="key, e.g. ABC" bind:value={slug} /></div>
    <Button variant="primary" type="submit">Add project</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete ${doomed?.name ?? 'project'}?`} description="Its milestones go with it. Issues in it stay but lose the project.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete project</Button>
  {/snippet}
</Dialog>

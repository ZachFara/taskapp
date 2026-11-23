<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { projects, projectStatuses, refreshAll } from '../../lib/stores.svelte';
  import type { ProjectStatus, ProjectStatusCategory } from '../../lib/bindings';
  import { glyphCategory } from '../../lib/projectFields';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  const categories: ProjectStatusCategory[] = ['Planned', 'Started', 'Paused', 'Completed', 'Cancelled'];
  let name = $state('');
  let category = $state<string>('Planned');
  let look = $state<{ icon: string | null; color: string | null }>({ icon: null, color: null });
  let error = $state('');
  let doomed = $state<ProjectStatus | null>(null);
  let target = $state<number | null>(null);

  const users = (s: ProjectStatus) => projects.filter((p) => p.status_id === s.id).length;
  const others = $derived(projectStatuses.filter((s) => s.id !== doomed?.id));

  onMount(() => refreshAll());

  // Every write goes through here so a refused write shows its message and the store stays in step.
  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  const add = () => write(async () => {
    await api.project.statusesCreate(name.trim(), category as ProjectStatusCategory, look.color ?? 'gray', look.icon);
    name = '';
    look = { icon: null, color: null };
  });
  const rename = (s: ProjectStatus, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== s.name) void write(() => api.project.statusesUpdate(s.id, next, null, null));
  };
  function move(i: number, to: number) {
    if (to < 0 || to >= projectStatuses.length) return;
    const order = projectStatuses.map((s) => s.id);
    order.splice(to, 0, ...order.splice(i, 1));
    void write(() => api.project.statusesReorder(order));
  }
  const confirmDelete = () => write(async () => {
    await api.project.statusesDelete(doomed!.id, target!);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="The stages a project moves through. These are separate from issue statuses; each belongs to a category that says what it means." />

  <ul class="es-list">
    {#each projectStatuses as s, i (s.id)}
      <li class="es-row">
        <StatusGlyph category={glyphCategory[s.category]} />
        <IconColorPicker label={`Icon and colour of ${s.name}`} value={{ icon: s.icon, color: s.color }} onchange={(v) => write(() => api.project.statusesUpdate(s.id, null, v.color, v.icon))} />
        <div class="es-name" onchange={(e) => rename(s, e)}><TextInput label={`Name of ${s.name}`} value={s.name} /></div>
        <span class="es-meta">{s.category} · {users(s)} {users(s) === 1 ? 'project' : 'projects'}</span>
        <div class="es-tools">
          <IconButton icon="arrow-up" label={`Move ${s.name} up`} disabled={i === 0} onclick={() => move(i, i - 1)} />
          <IconButton icon="arrow-down" label={`Move ${s.name} down`} disabled={i === projectStatuses.length - 1} onclick={() => move(i, i + 1)} />
          <IconButton icon="trash" label={`Delete ${s.name}`} disabled={projectStatuses.length < 2} onclick={() => { doomed = s; target = null; }} />
        </div>
      </li>
    {/each}
  </ul>

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <IconColorPicker label="New project status icon and colour" value={look} onchange={(v) => (look = v)} />
    <div class="es-name"><TextInput label="New project status name" placeholder="New project status" bind:value={name} /></div>
    <Select label="Category" bind:value={category} items={categories.map((c) => ({ value: c, label: c }))} />
    <Button variant="primary" type="submit">Add status</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog
  bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }}
  title={`Delete ${doomed?.name ?? 'status'}?`}
  description={`Its ${doomed ? users(doomed) : 0} projects need a new status first. Choose where they go.`}
>
  {#snippet children()}
    <div class="es-pick" role="group" aria-label="Move projects to">
      {#each others as o (o.id)}
        <button type="button" aria-label={`Move projects to ${o.name}`} aria-pressed={target === o.id} onclick={() => (target = o.id)}>{o.name}</button>
      {/each}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" disabled={target === null} onclick={confirmDelete}>Delete status</Button>
  {/snippet}
</Dialog>

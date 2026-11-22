<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Cycle } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  let cycles = $state<Cycle[]>([]);
  let name = $state('');
  let start = $state('');
  let end = $state('');
  let look = $state<{ icon: string | null; color: string | null }>({ icon: null, color: null });
  let error = $state('');
  let doomed = $state<Cycle | null>(null);

  const label = (c: Cycle) => c.name ?? `Cycle ${c.number}`;
  const load = async () => (cycles = await api.cycles.list());
  onMount(load);

  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await load();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  const add = () => write(async () => {
    await api.cycles.create(name.trim() || null, start, end, look.icon, look.color);
    name = start = end = '';
    look = { icon: null, color: null };
  });
  const edit = (c: Cycle, patch: Partial<Cycle>) => write(() => api.cycles.update({ ...c, ...patch }));
  const val = (e: Event) => (e.target as HTMLInputElement).value.trim();
  const remove = () => write(async () => {
    await api.cycles.delete(doomed!.id);
    doomed = null;
  });
</script>

<div class="es">
  <header class="es-head">
    <h2>Cycles</h2>
    <p>Fixed stretches of time, numbered in order. Cycles never overlap.</p>
  </header>

  {#if cycles.length}
    <ul class="es-list">
      {#each cycles as c (c.id)}
        <li class="es-row">
          <IconColorPicker label={`Icon and colour of ${label(c)}`} value={{ icon: c.icon, color: c.color }} onchange={(v) => edit(c, { icon: v.icon, color: v.color })} />
          <span class="es-meta">#{c.number}</span>
          <div class="es-name" onchange={(e) => edit(c, { name: val(e) || null })}><TextInput label={`Name of ${label(c)}`} placeholder="Unnamed" value={c.name ?? ''} /></div>
          <div class="date" onchange={(e) => edit(c, { start_date: val(e) })}><TextInput type="date" label={`Start of ${label(c)}`} value={c.start_date} /></div>
          <div class="date" onchange={(e) => edit(c, { end_date: val(e) })}><TextInput type="date" label={`End of ${label(c)}`} value={c.end_date} /></div>
          <div class="es-tools"><IconButton icon="trash" label={`Delete ${label(c)}`} onclick={() => (doomed = c)} /></div>
        </li>
      {/each}
    </ul>
  {:else}
    <EmptyState title="No cycles yet" hint="Add a start and end date to plan the first one." />
  {/if}

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <IconColorPicker label="New cycle icon and colour" value={look} onchange={(v) => (look = v)} />
    <div class="es-name"><TextInput label="Cycle name" placeholder="Name (optional)" bind:value={name} /></div>
    <div class="date"><TextInput type="date" label="Cycle start" bind:value={start} /></div>
    <div class="date"><TextInput type="date" label="Cycle end" bind:value={end} /></div>
    <Button variant="primary" type="submit">Add cycle</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete ${doomed ? label(doomed) : 'cycle'}?`} description="Issues in it stay but leave the cycle.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete cycle</Button>
  {/snippet}
</Dialog>

<style>
  .date { width: 150px; flex: none; }
</style>

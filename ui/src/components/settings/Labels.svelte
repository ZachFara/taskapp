<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Label } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  let labels = $state<Label[]>([]);
  let name = $state('');
  let look = $state<{ icon: string | null; color: string | null }>({ icon: null, color: null });
  let error = $state('');
  let doomed = $state<Label | null>(null);

  const load = async () => (labels = await api.labels.list());
  onMount(load);

  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await load();
    } catch (e) {
      const msg = (e as { message?: string }).message ?? String(e);
      // The backend reports a duplicate name as a raw UNIQUE constraint error.
      error = msg.includes('UNIQUE constraint') ? "A label with that name already exists" : msg;
    }
  }

  const add = () => write(async () => {
    await api.labels.create(name.trim(), look.color ?? 'gray', look.icon);
    name = '';
    look = { icon: null, color: null };
  });
  const rename = (l: Label, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== l.name) void write(() => api.labels.update({ ...l, name: next }));
  };
  const remove = () => write(async () => {
    await api.labels.delete(doomed!.id);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="Free-form tags you can put on any issue. Pick a colour and an icon so they read at a glance." />

  {#if labels.length}
    <ul class="es-list">
      {#each labels as l (l.id)}
        <li class="es-row">
          <IconColorPicker label={`Icon and colour of ${l.name}`} value={{ icon: l.icon, color: l.color }} onchange={(v) => write(() => api.labels.update({ ...l, icon: v.icon, color: v.color ?? l.color }))} />
          <div class="es-name" onchange={(e) => rename(l, e)}><TextInput label={`Name of ${l.name}`} value={l.name} /></div>
          <div class="es-tools"><IconButton icon="trash" label={`Delete ${l.name}`} onclick={() => (doomed = l)} /></div>
        </li>
      {/each}
    </ul>
  {:else}
    <EmptyState title="No labels yet" hint="Add one below, then use it from any issue." />
  {/if}

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <IconColorPicker label="New label icon and colour" value={look} onchange={(v) => (look = v)} />
    <div class="es-name"><TextInput label="Label name" placeholder="New label" bind:value={name} /></div>
    <Button variant="primary" type="submit">Add label</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete ${doomed?.name ?? 'label'}?`} description="It is removed from every issue that has it.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete label</Button>
  {/snippet}
</Dialog>

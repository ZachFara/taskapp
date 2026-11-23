<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Label } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import RowActions from '../../ui/RowActions.svelte';
  import { rowActs, rowUi } from '../../lib/rowUi.svelte';
  import { patch } from '../../lib/rowActions';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  let labels = $state<Label[]>([]);
  let name = $state('');
  let look = $state<{ icon: string | null; color: string | null }>({ icon: null, color: null });
  let error = $state('');

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
</script>

<div class="es">
  <SectionHead lede="Free-form tags you can put on any issue. Pick a colour and an icon so they read at a glance." />

  {#if labels.length}
    <ul class="es-list">
      {#each labels as l (l.id)}
        <li class="es-row">
          <RowActions label={l.name} actions={rowActs({ kind: 'label', id: l.id, name: l.name })}>
            <IconColorPicker label={`Icon and colour of ${l.name}`} value={{ icon: l.icon, color: l.color }} onchange={(v) => patch({ kind: 'label', id: l.id, name: l.name }, { icon: v.icon, color: v.color ?? l.color })} bind:open={() => rowUi.picking === `label:${l.id}`, (o) => (rowUi.picking = o ? `label:${l.id}` : null)} />
            <div class="es-name" onchange={(e) => rename(l, e)}><TextInput label={`Name of ${l.name}`} value={l.name} /></div>
          </RowActions>
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


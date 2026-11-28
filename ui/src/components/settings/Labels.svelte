<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Label, LabelGroup } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import Switch from '../../ui/Switch.svelte';
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
  let groups = $state<LabelGroup[]>([]);
  let groupName = $state('');
  let doomed = $state<LabelGroup | null>(null);
  let groupField = $state<HTMLElement>();

  const load = async () => ([labels, groups] = await Promise.all([api.labels.list(), api.label.groupsList()]));
  onMount(() => {
    // The command `label.group.new` brings the group name field into focus.
    const focus = () => groupField?.querySelector('input')?.focus();
    addEventListener('command:label.group.new', focus);
    void load();
    return () => removeEventListener('command:label.group.new', focus);
  });
  const groupItems = $derived([{ value: '', label: 'No group' }, ...groups.map((g) => ({ value: String(g.id), label: g.name }))]);

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
  const addGroup = () => write(async () => {
    await api.label.groupsCreate(groupName.trim());
    groupName = '';
  });
  const changeGroup = (g: LabelGroup, p: Partial<LabelGroup>) => write(() => api.label.groupsUpdate({ ...g, ...p }));
  // The dialog closes first, so a refusal shows in the page below it.
  const removeGroup = () => {
    const { id } = doomed!;
    doomed = null;
    return write(() => api.label.groupsDelete(id));
  };
  const rename = (l: Label, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== l.name) void write(() => api.labels.update({ ...l, name: next }));
  };
</script>

<div class="es">
  <SectionHead lede="Free-form tags you can put on any issue or project. Pick a colour and an icon so they read at a glance." />

  {#if labels.length}
    <ul class="es-list">
      {#each labels as l (l.id)}
        <li class="es-row">
          <RowActions label={l.name} actions={rowActs({ kind: 'label', id: l.id, name: l.name })}>
            <IconColorPicker label={`Icon and colour of ${l.name}`} value={{ icon: l.icon, color: l.color }} onchange={(v) => patch({ kind: 'label', id: l.id, name: l.name }, { icon: v.icon, color: v.color ?? l.color })} bind:open={() => rowUi.picking === `label:${l.id}`, (o) => (rowUi.picking = o ? `label:${l.id}` : null)} />
            <div class="es-name" onchange={(e) => rename(l, e)}><TextInput label={`Name of ${l.name}`} value={l.name} /></div>
            {#if groups.length}
              <Select label={`Group of ${l.name}`} items={groupItems} bind:value={() => String(l.group_id ?? ''), (v) => void write(() => api.labels.update({ ...l, group_id: v ? Number(v) : null }))} />
            {/if}
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
  <SectionHead lede="Groups sort labels into sets, like a Context group of @home and @phone. In an exclusive group an issue holds at most one label." />
  <ul class="es-list">
    {#each groups as g (g.id)}
      <li class="es-row">
        <div class="es-name"><TextInput label={`Name of group ${g.name}`} value={g.name} onchange={(v) => v.trim() && v !== g.name && changeGroup(g, { name: v })} /></div>
        <span class="es-meta">{labels.filter((l) => l.group_id === g.id).length} labels</span>
        <Switch label={`Exclusive: ${g.name}`} bind:checked={() => g.exclusive, (v) => void changeGroup(g, { exclusive: v })} />
        <div class="es-tools"><IconButton icon="trash" label={`Delete group ${g.name}`} onclick={() => (doomed = g)} /></div>
      </li>
    {/each}
  </ul>
  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void addGroup(); }}>
    <div class="es-name" bind:this={groupField}><TextInput label="Group name" placeholder="New group, e.g. Context" bind:value={groupName} /></div>
    <Button variant="primary" type="submit">Add group</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete group ${doomed?.name ?? ''}?`} description="Its labels stay, outside any group.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={removeGroup}>Delete group</Button>
  {/snippet}
</Dialog>


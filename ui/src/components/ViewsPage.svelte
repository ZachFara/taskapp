<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Layout, SavedView } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import NewViewDialog from './display/NewViewDialog.svelte';
  import { defaultTemplateId, setDefaultTemplate, templates, type ViewTemplate } from './display/templatesApi';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import ListRow from '../ui/ListRow.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Select from '../ui/Select.svelte';
  import TextInput from '../ui/TextInput.svelte';

  const layouts: { value: string; label: string }[] = (['List', 'Board', 'Table', 'Gantt', 'Calendar'] as Layout[]).map((l) => ({
    value: l,
    label: l === 'Gantt' ? 'Timeline' : l,
  }));

  let views = $state<SavedView[]>([]);
  let pinned = $state<Set<string>>(new Set());
  let loaded = $state(false);
  let doomed = $state<SavedView | null>(null);
  let confirming = $state(false);
  let creating = $state(false);

  // Templates are null until the backend exposes them; then the section is hidden.
  let tpls = $state<ViewTemplate[] | null>(null);
  let defaultId = $state<number | null>(null);
  let editing = $state<ViewTemplate | null>(null);
  let editOpen = $state(false);
  let draftName = $state('');
  let draftLayout = $state('List');
  let editLayout = $state(false);
  let doomedTpl = $state<ViewTemplate | null>(null);
  let confirmingTpl = $state(false);

  async function load() {
    const [list, pins] = await Promise.all([api.views.list(), api.pins.listResolved()]);
    views = list;
    pinned = new Set(pins.map((p) => p.route));
    loaded = true;
    [tpls, defaultId] = await Promise.all([templates().list(), defaultTemplateId()]);
  }

  async function remove() {
    if (!doomed) return;
    await api.views.delete(doomed.id);
    confirming = false;
    await load();
  }

  function openEdit(t: ViewTemplate, withLayout: boolean) {
    editing = t;
    editOpen = true;
    draftName = t.name;
    draftLayout = t.def.layout;
    editLayout = withLayout;
  }
  async function saveTemplate() {
    if (!editing) return;
    const next = draftName.trim();
    if (!next) return;
    await templates().update({ ...editing, name: next, def: { ...editing.def, layout: draftLayout as Layout } });
    editOpen = false;
    await load();
  }
  async function removeTemplate() {
    if (!doomedTpl) return;
    await templates().delete(doomedTpl.id);
    confirmingTpl = false;
    await load();
  }
  async function makeDefault(t: ViewTemplate) {
    await setDefaultTemplate(t.id);
    defaultId = t.id;
  }

  onMount(() => {
    load();
    // The command menu's "New view" lands here from another route (see defs/create/view.new.ts).
    const open = () => (creating = true);
    addEventListener('command:view.new', open);
    return () => removeEventListener('command:view.new', open);
  });
</script>

<PageHeader title="Views" icon="icon:layers">
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New view</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && views.length === 0}
    <EmptyState title="No saved views" hint="A view is a filter, grouping and layout you come back to.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={() => (creating = true)}>New view</Button>{/snippet}
    </EmptyState>
  {:else}
    <ul class="rows">
      {#each views as v (v.id)}
        <li class="row" data-view-id={v.id}>
          <ListRow onpick={() => navigate(`/view/${v.id}`)}>
            <EntityIcon icon={v.icon ?? 'icon:layers'} color={v.color} />
            <span class="name">{v.name}</span>
            <Pill>{v.def.layout}</Pill>
            {#if pinned.has(`/view/${v.id}`)}<Pill>Pinned</Pill>{/if}
          </ListRow>
          <IconButton icon="trash" label={`Delete ${v.name}`} onclick={() => ((doomed = v), (confirming = true))} />
        </li>
      {/each}
    </ul>
  {/if}

  {#if tpls}
    <h2 class="section">Templates</h2>
    {#if tpls.length === 0}
      <p class="hint">A template is a view definition new views start from. Use "Save as template" in a view's menu.</p>
    {:else}
      <ul class="rows">
        {#each tpls as t (t.id)}
          <li class="row" data-template-id={t.id}>
            <div class="trow">
              <EntityIcon icon={t.icon ?? 'icon:layers'} color={t.color} />
              <span class="name">{t.name}</span>
              <Pill>{t.def.layout}</Pill>
              {#if defaultId === t.id}<Pill>Default</Pill>{/if}
            </div>
            <IconButton icon="star" label="Set as default" disabled={defaultId === t.id} onclick={() => makeDefault(t)} />
            <IconButton icon="edit" label={`Rename ${t.name}`} onclick={() => openEdit(t, false)} />
            <IconButton icon="layers" label={`Edit ${t.name}`} onclick={() => openEdit(t, true)} />
            <IconButton icon="trash" label={`Delete ${t.name}`} onclick={() => ((doomedTpl = t), (confirmingTpl = true))} />
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</main>

<NewViewDialog bind:open={creating} />

<Dialog bind:open={confirming} title="Delete view?" description={doomed ? `"${doomed.name}" goes away. Its issues stay.` : undefined}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (confirming = false)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete view</Button>
  {/snippet}
</Dialog>

<Dialog bind:open={editOpen} title={editLayout ? 'Edit template' : 'Rename template'}>
  <div class="form">
    <TextInput label="Name" bind:value={draftName} />
    {#if editLayout}<Select label="Layout" items={layouts} bind:value={draftLayout} />{/if}
  </div>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (editOpen = false)}>Cancel</Button>
    <Button variant="primary" onclick={saveTemplate}>Save</Button>
  {/snippet}
</Dialog>

<Dialog bind:open={confirmingTpl} title="Delete template?" description={doomedTpl ? `"${doomedTpl.name}" goes away. Views made from it stay.` : undefined}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (confirmingTpl = false)}>Cancel</Button>
    <Button variant="danger" onclick={removeTemplate}>Delete template</Button>
  {/snippet}
</Dialog>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: var(--s-3); padding-right: var(--s-4); border-bottom: 1px solid var(--border); }
  .name { font-weight: 600; margin-right: auto; }
  .trow { flex: 1; display: flex; align-items: center; gap: var(--s-5); height: var(--row-h); padding: 0 var(--s-7); }
  .section { font-size: var(--fs-13); font-weight: 600; margin: var(--s-7) 0 var(--s-3); color: var(--muted); }
  .hint { color: var(--muted); margin: 0 0 var(--s-4); }
  .form { display: grid; gap: var(--s-4); }
</style>

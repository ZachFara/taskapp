<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Layout, Placement, SavedView, ViewFolder } from '../lib/bindings';
  import { buildRows } from '../lib/folderTree';
  import { navigate } from '../app/routes';
  import NewViewDialog from './display/NewViewDialog.svelte';
  import { defaultTemplateId, setDefaultTemplate, templates, type ViewTemplate } from './display/templatesApi';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import ListRow from '../ui/ListRow.svelte';
  import RowActions from '../ui/RowActions.svelte';
  import { rowActs, rowUi } from '../lib/rowUi.svelte';
  import { patch } from '../lib/rowActions';
  import IconColorPicker from './pickers/IconColorPicker.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Select from '../ui/Select.svelte';
  import TextInput from '../ui/TextInput.svelte';

  // The layout is stored as Gantt; users see it as Timeline.
  const layoutName = (l: string) => (l === 'Gantt' ? 'Timeline' : l);
  const layouts: { value: string; label: string }[] = (['List', 'Board', 'Table', 'Gantt', 'Calendar'] as Layout[]).map((l) => ({
    value: l,
    label: layoutName(l),
  }));

  let views = $state<SavedView[]>([]);
  let folders = $state<ViewFolder[]>([]);
  let places = $state<Placement[]>([]);
  // Folders first, nested, then each folder's views; the views of no folder come last.
  const tree = $derived(buildRows('view', folders, places, views, { placed: true }));
  const firstLoose = $derived(tree.findIndex((r) => r.type === 'item' && r.depth === 0));
  let pinned = $state<Set<string>>(new Set());
  let loaded = $state(false);
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
    const [list, pins, fs, pl] = await Promise.all([api.views.list(), api.pins.listResolved(), api.folders.list(), api.folders.items()]);
    views = list;
    folders = fs;
    places = pl;
    pinned = new Set(pins.map((p) => p.route));
    loaded = true;
    [tpls, defaultId] = await Promise.all([templates().list(), defaultTemplateId()]);
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
    addEventListener('rows:changed', load);
    return () => {
      removeEventListener('command:view.new', open);
      removeEventListener('rows:changed', load);
    };
  });
</script>

{#snippet viewRow(v: SavedView)}
  <li class="row" data-view-id={v.id}>
    <RowActions label={v.name} actions={rowActs({ kind: 'view', id: v.id, name: v.name }, pinned.has(`/view/${v.id}`))}>
      <IconColorPicker label={`Icon and colour of ${v.name}`} value={{ icon: v.icon ?? 'icon:layers', color: v.color }} onchange={(c) => patch({ kind: 'view', id: v.id, name: v.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `view:${v.id}`, (o) => (rowUi.picking = o ? `view:${v.id}` : null)} />
      <span class="grow">
        <ListRow onpick={() => navigate(`/view/${v.id}`)}>
          <span class="name">{v.name}</span>
          <Pill>{layoutName(v.def.layout)}</Pill>
          {#if pinned.has(`/view/${v.id}`)}<Pill>Pinned</Pill>{/if}
        </ListRow>
      </span>
    </RowActions>
  </li>
{/snippet}

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
    {#each tree as r, i (r.type === 'folder' ? `f${r.folder.id}` : `v${r.item.id}`)}
      {#if r.type === 'folder'}
        <h2 class="section" data-folder-id={r.folder.id} style:margin-left={`calc(${r.depth} * var(--s-8))`}>{r.folder.name}</h2>
      {:else}
        {#if folders.length && i === firstLoose}<h2 class="section">No folder</h2>{/if}
        <ul class="rows" style:margin-left={`calc(${r.depth} * var(--s-8))`}>{@render viewRow(r.item)}</ul>
      {/if}
    {/each}
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
              <Pill>{layoutName(t.def.layout)}</Pill>
              {#if defaultId === t.id}<Pill>Default</Pill>{/if}
            </div>
            <div class="tools">
              <IconButton icon="star" label="Set as default" disabled={defaultId === t.id} onclick={() => makeDefault(t)} />
              <IconButton icon="edit" label={`Rename ${t.name}`} onclick={() => openEdit(t, false)} />
              <IconButton icon="layers" label={`Edit ${t.name}`} onclick={() => openEdit(t, true)} />
              <IconButton icon="trash" label={`Delete ${t.name}`} onclick={() => ((doomedTpl = t), (confirmingTpl = true))} />
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</main>

<NewViewDialog bind:open={creating} />


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
  .tools { display: flex; gap: var(--s-1); opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  .row:hover .tools, .row:focus-within .tools { opacity: 1; }
  .trow { flex: 1; display: flex; align-items: center; gap: var(--s-5); height: var(--row-h); padding: 0 var(--s-7); }
  .section { font-size: var(--fs-13); font-weight: 600; margin: var(--s-7) 0 var(--s-3); color: var(--muted); }
  .hint { color: var(--muted); margin: 0 0 var(--s-4); }
  .form { display: grid; gap: var(--s-4); }
</style>

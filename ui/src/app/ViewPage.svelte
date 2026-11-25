<script lang="ts">
  import { selection } from '../lib/selection.svelte';
  import { onMount, type Component } from 'svelte';
  import type { Layout, SavedView, ViewDef } from '../lib/bindings';
  import { api } from '../lib/api';
  import { issues, refreshAll, selectedIssueId } from '../lib/stores.svelte';
  import { announce } from '../components/undoState.svelte';
  import ViewEditor from '../components/ViewEditor.svelte';
  import InsightsPanel from '../components/insights/InsightsPanel.svelte';
  import CommandButton from '../components/CommandButton.svelte';
  import BoardView from '../components/views/BoardView.svelte';
  import CalendarView from '../components/views/CalendarView.svelte';
  import GanttView from '../components/views/GanttView.svelte';
  import ListView from '../components/views/ListView.svelte';
  import TableView from '../components/views/TableView.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import IconColorPicker from '../components/pickers/IconColorPicker.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Popover from '../ui/Popover.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { navigate } from './routes';
  import QuickFilter from '../components/filters/QuickFilter.svelte';
  import { combineFilters } from '../components/filters/ephemeral';
  import { buildFilter, newChip, type Chip } from '../components/filters/model';
  import { shortcutFor } from '../lib/commands/registry';
  import NewViewDialog from '../components/display/NewViewDialog.svelte';
  import { templates } from '../components/display/templatesApi';

  // `layout` in the URL overrides the saved one until the user picks a tab, so the harness can shoot any layout.
  let { params }: { params: { id: string; layout?: string } } = $props();
  const byLayout: Record<Layout, Component<any>> = {
    List: ListView, Table: TableView, Board: BoardView, Gantt: GanttView, Calendar: CalendarView,
  };
  const layoutTabs = $derived([
    { id: 'List', label: 'List', icon: 'list', cmd: 'list' }, { id: 'Board', label: 'Board', icon: 'board', cmd: 'board' },
    { id: 'Table', label: 'Table', icon: 'table', cmd: 'table' }, { id: 'Gantt', label: 'Timeline', icon: 'timeline', cmd: 'timeline' },
    { id: 'Calendar', label: 'Calendar', icon: 'calendar', cmd: 'calendar' },
  ].map(({ cmd, ...t }) => ({ ...t, shortcut: shortcutFor(`view.layout.${cmd}`) })));
  const SAVE_DELAY = 400;

  let view = $state.raw<SavedView | null>(null);
  let def = $state.raw<ViewDef | null>(null);
  let name = $state('');
  let override = $state<string | null>(null);
  let missing = $state(false);
  let pinId = $state<number | null>(null);
  let rows = $state<number | null>(null);
  let clearSignal = $state(0);
  let renaming = $state(false);
  let draft = $state('');
  let confirmDelete = $state(false);
  let insights = $state(false);
  let iconOpen = $state(false);
  let creating = $state(false);
  // Quick filters narrow what this page shows. They are never saved unless "Save to view" keeps them.
  let quickOpen = $state(false);
  let quickText = $state('');
  let quickChips = $state<Chip[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function changeLook(c: { icon: string | null; color: string | null }) {
    if (!view || !def) return;
    await api.views.save(view.id, name, def, c.icon, c.color);
    await load();
  }

  async function load() {
    const found = (await api.views.list()).find((v) => String(v.id) === params.id) ?? null;
    view = found;
    def = found ? structuredClone($state.snapshot(found.def)) : null;
    name = found?.name ?? '';
    override = params.layout ?? null;
    quickOpen = false;
    quickText = '';
    quickChips = [];
    missing = found === null;
    pinId = (await api.pins.listResolved()).find((p) => p.pin.kind === 'View' && p.pin.target === params.id)?.pin.id ?? null;
  }
  $effect(() => {
    params.id;
    clearTimeout(timer);
    load();
  });

  const layout = $derived<Layout>((override as Layout) ?? def?.layout ?? 'List');
  // Commands scope their keys by layout; the timeline does not announce itself, and tabs can change without a route change.
  $effect(() => selection.setLayout(({ List: 'list', Table: 'table', Board: 'board', Gantt: 'timeline', Calendar: 'calendar' } as const)[layout]));
  const live = $derived(view && def ? { ...view, name, def: { ...def, layout } } : null);
  const quickFilter = $derived(combineFilters(quickText.trim() ? { Cond: { field: { Fixed: 'Title' }, op: 'Contains', value: quickText.trim() } } : null, buildFilter(null, quickChips)));
  // What the layouts get: the saved view plus the quick filter. Only the saved part is ever written.
  const shown = $derived(live && { ...live, def: { ...live.def, filter: combineFilters(live.def.filter, quickFilter) } });
  const filtered = $derived(shown?.def.filter != null);
  function closeQuick() {
    quickOpen = false;
    quickText = '';
    quickChips = [];
  }
  // Keeps the quick filter as part of the saved view, through the same save as a chip edit.
  let editorKey = $state(0); // the editor reads the saved filter once, so it is remounted to show the kept chips
  function saveQuick() {
    if (!def || quickFilter === null) return;
    change({ filter: combineFilters(def.filter, quickFilter) });
    closeQuick();
    editorKey++;
  }

  // Debounced: a burst of chip edits is one save.
  // A filter change changes what the user sees, so its save is announced with an Undo; display toggles are not.
  let loud = false;
  const save = () => {
    clearTimeout(timer);
    // Taken now, not when the timer fires: by then the page may show another view.
    const [v, d, n] = [view, def, name];
    timer = setTimeout(async () => {
      if (!v || !d) return;
      const say = loud;
      loud = false;
      await api.views.save(v.id, n, d, v.icon, v.color);
      if (say) announce(`Changed filter of ${n}`);
    }, SAVE_DELAY);
  };
  function change(patch: Partial<ViewDef>) {
    if (!def) return;
    def = { ...def, ...patch };
    if ('filter' in patch) loud = true;
    save();
  }
  const setLayout = (l: string) => {
    override = null;
    change({ layout: l as Layout });
  };
  async function rename() {
    renaming = false;
    const next = draft.trim();
    if (!next || !view || next === name) return;
    name = next;
    await api.views.save(view.id, next, def!, view.icon, view.color);
    announce(`Renamed view to ${next}`);
    await refreshAll();
  }
  async function togglePin() {
    if (!view) return;
    if (pinId !== null) await api.pins.unpin(pinId);
    pinId = pinId === null ? (await api.pins.pin('View', String(view.id), name)).id : null;
    await refreshAll();
  }
  async function saveAsNew() {
    if (!view || !def) return;
    const v = await api.views.save(null, `${name} copy`, { ...def, layout }, view.icon, view.color);
    await refreshAll();
    navigate(`/view/${v.id}`);
  }
  // Templates are read from the stored view, so the pending debounced save goes first.
  async function saveAsTemplate() {
    if (!view || !def) return;
    clearTimeout(timer);
    await api.views.save(view.id, name, def, view.icon, view.color);
    await templates().saveAsTemplate(view.id, null);
  }
  async function resetToTemplate() {
    if (!view) return;
    clearTimeout(timer);
    await templates().resetToTemplate(view.id);
    await load();
    await refreshAll();
  }
  async function remove() {
    if (!view) return;
    confirmDelete = false;
    await api.views.delete(view.id);
    await refreshAll();
    navigate('/views');
  }

  // Count for the empty state; the layout components run the same query for their own rows.
  $effect(() => {
    const d = shown?.def;
    if (d) api.views.query(d).then((r) => (rows = r.length));
  });

  onMount(() => {
    const on = (type: string, fn: (e: Event) => void) => {
      addEventListener(type, fn);
      return () => removeEventListener(type, fn);
    };
    const offs = [
      on('command:view.layout', (e) => setLayout((e as CustomEvent<string>).detail)),
      on('command:view.pin', togglePin),
      on('command:view.rename', () => ((draft = name), (renaming = true))),
      on('command:view.duplicate', saveAsNew),
      on('command:view.saveAsNew', saveAsNew),
      on('command:view.delete', () => (confirmDelete = true)),
      on('command:view.saveAsTemplate', saveAsTemplate),
      on('command:view.resetToTemplate', resetToTemplate),
      on('command:insights.toggle', () => (insights = !insights)),
      on('command:view.quickFilter', () => (quickOpen ? closeQuick() : (quickOpen = true))),
      on('command:icon.change', () => (iconOpen = true)),
    ];
    return () => {
      offs.forEach((f) => f());
      clearTimeout(timer);
    };
  });

  // Every view marks its issue rows with the key (or, in the Gantt, the id); a click on one opens the panel.
  function pick(e: MouseEvent) {
    if (e.metaKey || e.ctrlKey || e.shiftKey) return; // selecting rows is not opening one
    const el = (e.target as Element).closest<HTMLElement>('[data-key],[data-issue]');
    const tag = el?.dataset.key ?? el?.dataset.issue;
    const hit = issues.find((i) => i.key === tag || String(i.id) === tag);
    if (hit) selectedIssueId.value = hit.id;
  }
  function clearFilters() {
    clearSignal++;
    closeQuick();
    change({ filter: null });
  }
  const Shown = $derived(byLayout[layout]);
</script>

{#if live}
  {#key params.id}
    <div class="page">
    <PageHeader title={name} icon={view?.icon ?? 'icon:layers'} color={view?.color ?? undefined}>
      {#snippet iconEdit()}<IconColorPicker label={`Icon and colour of ${name}`} value={{ icon: view?.icon ?? 'icon:layers', color: view?.color ?? null }} onchange={changeLook} bind:open={iconOpen} />{/snippet}
      {#snippet tabs()}<Tabs tabs={layoutTabs} bind:value={() => layout, setLayout} />{/snippet}
      {#snippet filters()}{#key `${live.id}:${editorKey}`}<ViewEditor def={live.def} onchange={change} {clearSignal} />{/key}{/snippet}
      {#snippet actions()}
        <Popover bind:open={renaming}>
          {#snippet trigger(props)}<span {...props} class="trig"><IconButton icon="edit" label="Rename view" shortcut={shortcutFor('view.rename')} onclick={() => (draft = name)} /></span>{/snippet}
          <form onsubmit={(e) => { e.preventDefault(); rename(); }}>
            <TextInput label="View name" bind:value={draft} />
            <div class="row"><Button variant="primary" type="submit">Rename</Button></div>
          </form>
        </Popover>
        <IconButton icon="filter" label="Quick filter" shortcut={shortcutFor('view.quickFilter')} pressed={quickOpen} onclick={() => (quickOpen ? closeQuick() : (quickOpen = true))} />
        <IconButton icon="target" label="Insights" shortcut="shift+i" pressed={insights} onclick={() => (insights = !insights)} />
        <IconButton icon="pin" label={pinId === null ? 'Pin view' : 'Unpin view'} shortcut={shortcutFor('view.pin')} pressed={pinId !== null} onclick={togglePin} />
        <Menu>
          {#snippet trigger(props)}<span {...props} class="trig"><IconButton icon="more" label="More" /></span>{/snippet}
          <MenuItem label="Save as new view" icon="copy" onSelect={saveAsNew} />
          <MenuItem label="New view..." icon="plus" onSelect={() => (creating = true)} />
          <MenuItem label="Save as template" icon="bookmark" onSelect={saveAsTemplate} />
          {#if view?.template_id != null}
            <MenuItem label="Reset to template" icon="undo" onSelect={resetToTemplate} />
          {/if}
          <MenuItem label="Delete view" icon="trash" danger onSelect={() => (confirmDelete = true)} />
        </Menu>
        <CommandButton command="issue.create" variant="primary" />
      {/snippet}
    </PageHeader>
    {#if quickOpen}
      <QuickFilter bind:text={quickText} bind:chips={quickChips} saveable={quickFilter !== null} onsave={saveQuick} onclose={closeQuick} />
    {/if}
    <div class="cols">
    <main class="body">
      {#if rows === 0 && filtered}
        <EmptyState title="Nothing matches these filters" hint="Loosen a filter or clear them all to see every issue in this view.">
          {#snippet action()}<Button onclick={clearFilters}>Clear filters</Button>{/snippet}
        </EmptyState>
      {:else}
        <div class="pane" role="presentation" onclick={pick}><Shown view={shown} embedded /></div>
      {/if}
    </main>
    {#if insights && view}<InsightsPanel kind="view" id={view.id} title={name} onclose={() => (insights = false)} />{/if}
    </div>
    </div>
  {/key}
  <NewViewDialog bind:open={creating} current={live} />
  <Dialog bind:open={confirmDelete} title={`Delete "${name}"?`} description="The saved view goes away. Issues are not touched.">
    {#snippet actions()}<Button variant="danger" onclick={remove}>Delete</Button>{/snippet}
  </Dialog>
{:else if missing}
  <EmptyState title="No such view" hint="It may have been deleted." />
{:else}
  <div class="load"><Skeleton height="var(--s-8)" /><Skeleton /><Skeleton /></div>
{/if}

<style>
  /* The page is as tall as the sheet, so a view can fill it and scroll inside itself. */
  .page { display: flex; flex-direction: column; height: 100%; }
  .cols { flex: 1; min-height: 0; display: flex; }
  .body { flex: 1; min-height: 0; overflow: auto; border-top: 1px solid var(--border); }
  .pane { height: 100%; }
  .trig { display: inline-flex; }
  .row { display: flex; justify-content: flex-end; margin-top: var(--s-4); }
  .load { display: grid; gap: var(--s-4); padding: var(--s-7); }
</style>

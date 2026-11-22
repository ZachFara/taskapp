<script lang="ts">
  import { onMount, type Component } from 'svelte';
  import type { Layout, SavedView, ViewDef } from '../lib/bindings';
  import { api } from '../lib/api';
  import { issues, refreshAll, selectedIssueId } from '../lib/stores.svelte';
  import ViewEditor from '../components/ViewEditor.svelte';
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
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Popover from '../ui/Popover.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { navigate } from './routes';

  // `layout` in the URL overrides the saved one until the user picks a tab, so the harness can shoot any layout.
  let { params }: { params: { id: string; layout?: string } } = $props();
  const byLayout: Record<Layout, Component<any>> = {
    List: ListView, Table: TableView, Board: BoardView, Gantt: GanttView, Calendar: CalendarView,
  };
  const layoutTabs = [
    { id: 'List', label: 'List', icon: 'list' }, { id: 'Board', label: 'Board', icon: 'board' },
    { id: 'Table', label: 'Table', icon: 'table' }, { id: 'Gantt', label: 'Timeline', icon: 'timeline' },
    { id: 'Calendar', label: 'Calendar', icon: 'calendar' },
  ];
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
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load() {
    const found = (await api.views.list()).find((v) => String(v.id) === params.id) ?? null;
    view = found;
    def = found ? structuredClone($state.snapshot(found.def)) : null;
    name = found?.name ?? '';
    override = params.layout ?? null;
    missing = found === null;
    pinId = (await api.pins.listResolved()).find((p) => p.pin.kind === 'View' && p.pin.target === params.id)?.pin.id ?? null;
  }
  $effect(() => {
    params.id;
    clearTimeout(timer);
    load();
  });

  const layout = $derived<Layout>((override as Layout) ?? def?.layout ?? 'List');
  const live = $derived(view && def ? { ...view, name, def: { ...def, layout } } : null);
  const filtered = $derived(def?.filter != null);

  // Debounced: a burst of chip edits is one save.
  const save = () => {
    clearTimeout(timer);
    timer = setTimeout(() => view && def && api.views.save(view.id, name, def, view.icon, view.color), SAVE_DELAY);
  };
  function change(patch: Partial<ViewDef>) {
    if (!def) return;
    def = { ...def, ...patch };
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
  async function remove() {
    if (!view) return;
    confirmDelete = false;
    await api.views.delete(view.id);
    await refreshAll();
    navigate('/views');
  }

  // Count for the empty state; the layout components run the same query for their own rows.
  $effect(() => {
    const d = live?.def;
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
      on('command:view.saveAsNew', saveAsNew),
      on('command:view.delete', () => (confirmDelete = true)),
    ];
    return () => {
      offs.forEach((f) => f());
      clearTimeout(timer);
    };
  });

  // Every view marks its issue rows with the key (or, in the Gantt, the id); a click on one opens the panel.
  function pick(e: MouseEvent) {
    const el = (e.target as Element).closest<HTMLElement>('[data-key],[data-issue]');
    const tag = el?.dataset.key ?? el?.dataset.issue;
    const hit = issues.find((i) => i.key === tag || String(i.id) === tag);
    if (hit) selectedIssueId.value = hit.id;
  }
  function clearFilters() {
    clearSignal++;
    change({ filter: null });
  }
  const Shown = $derived(byLayout[layout]);
</script>

{#if live}
  {#key params.id}
    <PageHeader title={name} icon={view?.icon ?? 'icon:layers'} color={view?.color ?? undefined}>
      {#snippet tabs()}<Tabs tabs={layoutTabs} bind:value={() => layout, setLayout} />{/snippet}
      {#snippet filters()}<ViewEditor def={live.def} onchange={change} {clearSignal} />{/snippet}
      {#snippet actions()}
        <Popover bind:open={renaming}>
          {#snippet trigger(props)}<span {...props} class="trig"><IconButton icon="edit" label="Rename view" onclick={() => (draft = name)} /></span>{/snippet}
          <form onsubmit={(e) => { e.preventDefault(); rename(); }}>
            <TextInput label="View name" bind:value={draft} />
            <div class="row"><Button variant="primary" type="submit">Rename</Button></div>
          </form>
        </Popover>
        <IconButton icon="pin" label={pinId === null ? 'Pin view' : 'Unpin view'} pressed={pinId !== null} onclick={togglePin} />
        <Menu>
          {#snippet trigger(props)}<span {...props} class="trig"><IconButton icon="more" label="More" /></span>{/snippet}
          <MenuItem label="Save as new view" icon="copy" onSelect={saveAsNew} />
          <MenuItem label="Delete view" icon="trash" danger onSelect={() => (confirmDelete = true)} />
        </Menu>
        <CommandButton command="issue.create" variant="primary" />
      {/snippet}
    </PageHeader>
    <main class="body">
      {#if rows === 0 && filtered}
        <EmptyState title="Nothing matches these filters" hint="Loosen a filter or clear them all to see every issue in this view.">
          {#snippet action()}<Button onclick={clearFilters}>Clear filters</Button>{/snippet}
        </EmptyState>
      {:else}
        <div role="presentation" onclick={pick}><Shown view={live} /></div>
      {/if}
    </main>
  {/key}
  <Dialog bind:open={confirmDelete} title={`Delete "${name}"?`} description="The saved view goes away. Issues are not touched.">
    {#snippet actions()}<Button variant="danger" onclick={remove}>Delete</Button>{/snippet}
  </Dialog>
{:else if missing}
  <EmptyState title="No such view" hint="It may have been deleted." />
{:else}
  <div class="load"><Skeleton height="var(--s-8)" /><Skeleton /><Skeleton /></div>
{/if}

<style>
  .body { flex: 1; min-height: 0; overflow: auto; border-top: 1px solid var(--border); }
  .trig { display: inline-flex; }
  .row { display: flex; justify-content: flex-end; margin-top: var(--s-4); }
  .load { display: grid; gap: var(--s-4); padding: var(--s-7); }
</style>

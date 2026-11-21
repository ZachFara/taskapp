<script lang="ts">
  import type { Component } from 'svelte';
  import type { Layout, SavedView } from '../lib/bindings';
  import { api } from '../lib/api';
  import { issues, refreshAll, selectedIssueId } from '../lib/stores.svelte';
  import ViewEditor from '../components/ViewEditor.svelte';
  import BoardView from '../components/views/BoardView.svelte';
  import CalendarView from '../components/views/CalendarView.svelte';
  import GanttView from '../components/views/GanttView.svelte';
  import ListView from '../components/views/ListView.svelte';
  import TableView from '../components/views/TableView.svelte';

  // `layout` in the URL overrides the saved one, so the harness can shoot any layout of any view.
  let { params }: { params: { id: string; layout?: string } } = $props();
  const byLayout: Record<Layout, Component<any>> = {
    List: ListView, Table: TableView, Board: BoardView, Gantt: GanttView, Calendar: CalendarView,
  };
  // raw: ViewEditor clones the view, which a reactive proxy cannot survive.
  let view = $state.raw<SavedView | null>(null);
  let missing = $state(false);
  let editing = $state(false);
  function load() {
    return api.views.list().then((vs) => {
      view = vs.find((v) => String(v.id) === params.id) ?? null;
      missing = view === null;
    });
  }
  $effect(() => {
    params.id;
    editing = false;
    load();
  });
  async function saved() {
    editing = false;
    await load();
    await refreshAll();
  }
  // Every view marks its issue rows with the key (or, in the Gantt, the id); a click on one opens the panel.
  function pick(e: MouseEvent) {
    const el = (e.target as Element).closest<HTMLElement>('[data-key],[data-issue]');
    const tag = el?.dataset.key ?? el?.dataset.issue;
    const hit = issues.find((i) => i.key === tag || String(i.id) === tag);
    if (hit) selectedIssueId.value = hit.id;
  }
  const Shown = $derived(view && byLayout[(params.layout as Layout) ?? view.def.layout]);
</script>

{#if Shown && view}
  <button type="button" onclick={() => (editing = !editing)}>Edit view</button>
  {#if editing}<ViewEditor {view} onsaved={saved} />{/if}
  <div role="presentation" onclick={pick}><Shown {view} /></div>
{:else if missing}
  <p>No saved view {params.id}</p>
{/if}

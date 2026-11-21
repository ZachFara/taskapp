<script lang="ts">
  import type { Component } from 'svelte';
  import type { Layout, SavedView } from '../lib/bindings';
  import { api } from '../lib/api';
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
  let view = $state<SavedView | null>(null);
  let missing = $state(false);
  $effect(() => {
    api.views.list().then((vs) => {
      view = vs.find((v) => String(v.id) === params.id) ?? null;
      missing = view === null;
    });
  });
  const Shown = $derived(view && byLayout[(params.layout as Layout) ?? view.def.layout]);
</script>

{#if Shown && view}
  <Shown {view} />
{:else if missing}
  <p>No saved view {params.id}</p>
{/if}

<script lang="ts">
  import type { Component } from 'svelte';
  import type { FieldRef, Filter, Layout, SavedView, ViewDef } from '../../lib/bindings';
  import { issues, selectedIssueId } from '../../lib/stores.svelte';
  import { clearViewContext, setViewContext } from '../../lib/viewContext.svelte';
  import BoardView from '../views/BoardView.svelte';
  import CalendarView from '../views/CalendarView.svelte';
  import GanttView from '../views/GanttView.svelte';
  import ListView from '../views/ListView.svelte';
  import TableView from '../views/TableView.svelte';

  // The page's toolbar owns filter, group and sort; this only turns them into a view of the project's issues.
  let { projectId, layout, def, onsort }: { projectId: number; layout: Layout; def: ViewDef; onsort?: (field: FieldRef) => void } = $props();
  const byLayout: Record<Layout, Component<any>> = { List: ListView, Table: TableView, Board: BoardView, Gantt: GanttView, Calendar: CalendarView };

  const inProject = (f: Filter | null): Filter => {
    const mine: Filter = { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: projectId } };
    return f ? { And: [mine, f] } : mine;
  };

  // raw: the views clone the def, which a reactive proxy cannot survive.
  const view = $derived.by<SavedView>(() => ({
    id: -projectId,
    name: 'Project',
    position: 0,
    icon: null,
    color: null,
    def: { ...def, layout, filter: inProject(def.filter) },
  }));
  const Shown = $derived(byLayout[layout]);
  $effect(() => {
    setViewContext({ inView: true, filter: view.def.filter, viewDef: view.def });
    return clearViewContext;
  });

  // Same pick rule as a saved view: rows are tagged with the key (or the id in the Gantt).
  function pick(e: MouseEvent) {
    const el = (e.target as Element).closest<HTMLElement>('[data-key],[data-issue]');
    const tag = el?.dataset.key ?? el?.dataset.issue;
    const hit = issues.find((i) => i.key === tag || String(i.id) === tag);
    if (hit) selectedIssueId.value = hit.id;
  }
</script>

{#key `${layout}:${projectId}:${JSON.stringify(def)}`}
  <div class="issues" role="presentation" onclick={pick}><Shown {view} {onsort} /></div>
{/key}

<style>
  .issues { height: 100%; min-height: 0; }
</style>

<script lang="ts">
  import type { Component } from 'svelte';
  import type { FieldRef, Layout, SavedView } from '../../lib/bindings';
  import { issues, selectedIssueId } from '../../lib/stores.svelte';
  import BoardView from '../views/BoardView.svelte';
  import CalendarView from '../views/CalendarView.svelte';
  import GanttView from '../views/GanttView.svelte';
  import ListView from '../views/ListView.svelte';
  import TableView from '../views/TableView.svelte';

  // Layout and grouping live in the page header; this only turns them into a view of the project's issues.
  let { projectId, layout, groupBy = null }: { projectId: number; layout: Layout; groupBy?: FieldRef | null } = $props();
  const byLayout: Record<Layout, Component<any>> = { List: ListView, Table: TableView, Board: BoardView, Gantt: GanttView, Calendar: CalendarView };

  // raw: the views clone the def, which a reactive proxy cannot survive.
  const view = $derived.by<SavedView>(() => ({
    id: -projectId,
    name: 'Project',
    position: 0,
    icon: null,
    color: null,
    def: {
      filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: projectId } },
      sorts: [],
      group_by: groupBy,
      layout,
      columns: [],
      include_archived: false,
    },
  }));
  const Shown = $derived(byLayout[layout]);

  // Same pick rule as a saved view: rows are tagged with the key (or the id in the Gantt).
  function pick(e: MouseEvent) {
    const el = (e.target as Element).closest<HTMLElement>('[data-key],[data-issue]');
    const tag = el?.dataset.key ?? el?.dataset.issue;
    const hit = issues.find((i) => i.key === tag || String(i.id) === tag);
    if (hit) selectedIssueId.value = hit.id;
  }
</script>

{#key `${layout}:${projectId}:${JSON.stringify(groupBy)}`}
  <div class="issues" role="presentation" onclick={pick}><Shown {view} /></div>
{/key}

<style>
  .issues { height: 100%; min-height: 0; }
</style>

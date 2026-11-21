<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, ViewDef } from '../../lib/bindings';

  let { projectId }: { projectId: number } = $props();
  let rows = $state<Issue[]>([]);

  // Plain list until the shared view components (T13 to T15) can take a fixed filter.
  $effect(() => {
    const def: ViewDef = {
      filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: projectId } },
      sorts: [],
      group_by: null,
      layout: 'List',
      columns: [],
      include_archived: false,
    };
    api.views.query(def).then((r) => (rows = r));
  });
</script>

<ul>
  {#each rows as issue (issue.id)}
    <li data-issue-key={issue.key}>{issue.key} {issue.title}</li>
  {:else}
    <li>No issues in this project</li>
  {/each}
</ul>

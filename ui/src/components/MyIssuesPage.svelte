<script lang="ts">
  import type { Filter, SavedView } from '../lib/bindings';
  import PageHeader from '../ui/PageHeader.svelte';
  import ListView from './views/ListView.svelte';

  // Single-user app: "my" is every issue that is not archived and not finished, grouped by status.
  const open: Filter = { Cond: { field: { Fixed: 'StatusCategory' }, op: 'NotIn', value: ['Completed', 'Canceled'] } };
  const view: SavedView = {
    id: 0,
    name: 'My issues',
    position: 0,
    icon: null,
    color: null,
    def: { filter: open, sorts: [], group_by: { Fixed: 'Status' }, layout: 'List', columns: [], include_archived: false },
  };
</script>

<PageHeader title="My issues" icon="icon:user" />
<ListView {view} />

<script lang="ts">
  import { rowUi } from '../lib/rowUi.svelte';
  import { patch, remove } from '../lib/rowActions';
  import RowDialog from './RowDialog.svelte';

  // Mounted once by the shell: every row action's rename or delete dialog opens here.
  const d = $derived(rowUi.dialog);
  const text = { view: 'Its issues stay.', project: 'Its milestones go with it. Issues in it stay but lose the project.', cycle: 'Issues in it stay but leave the cycle.', label: 'Issues keep their other labels.', template: 'Issues made from it stay.' } as const;
  function confirm(v: string) {
    const cur = rowUi.dialog;
    rowUi.dialog = null;
    if (!cur) return;
    if (cur.mode === 'rename') {
      if (v) void patch(cur.target, { name: v });
    } else void remove(cur.target);
  }
</script>

<RowDialog
  bind:open={() => rowUi.dialog !== null, (o) => { if (!o) rowUi.dialog = null; }}
  mode={d?.mode ?? 'rename'}
  title={d?.mode === 'delete' ? `Delete ${d.target.name}?` : `Rename ${d?.target.name ?? ''}`}
  value={d?.target.name ?? ''}
  description={d?.mode === 'delete' ? `"${d.target.name}" goes away. ${text[d.target.kind]}` : undefined}
  onconfirm={confirm}
/>

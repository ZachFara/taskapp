import { defineCommand } from '../../registry';
import { listState } from '../../../listState.svelte';

defineCommand({
  id: 'list.title.edit',
  label: 'Rename issue',
  icon: 'edit',
  group: 'List',
  shortcut: ['enter', 'f2'],
  scope: 'list',
  when: (ctx) => ctx.focusedIssueId !== null && listState.editing === null && listState.ids.includes(ctx.focusedIssueId),
  run: (ctx) => {
    listState.editing = ctx.focusedIssueId;
  },
});

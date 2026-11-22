import { defineCommand } from '../../registry';
import { listState } from '../../../listState.svelte';

defineCommand({
  id: 'list.title.edit',
  label: 'Rename issue',
  icon: 'edit',
  group: 'Issue',
  shortcut: 'f2',
  scope: 'list',
  when: (ctx) => ctx.focusedIssueId !== null && listState.editing === null && listState.ids.includes(ctx.focusedIssueId),
  run: (ctx) => {
    listState.editing = ctx.focusedIssueId;
  },
});

import { defineCommand } from '../../registry';
import { selection } from '../../../selection.svelte';
import { selectedIssueId } from '../../../stores.svelte';

// Opens the side panel over the current view; focus stays in the list so j and k make the panel follow.
defineCommand({
  id: 'move.open',
  label: 'Open issue',
  icon: 'panel',
  group: 'Issue',
  shortcut: 'enter',
  when: (ctx) => ctx.focusedIssueId !== null && !ctx.route.startsWith('/issue/'),
  run: () => {
    if (selection.focused !== null) selectedIssueId.value = selection.focused;
  },
});

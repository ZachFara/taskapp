import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.left',
  label: 'Previous column',
  icon: 'board',
  group: 'Navigation',
  shortcut: 'h',
  run: () => {
    selection.moveColumn(-1);
  },
});

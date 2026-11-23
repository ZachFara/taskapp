import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.left',
  menu: false,
  label: 'Previous column',
  icon: 'board',
  group: 'Navigation',
  scope: 'board',
  shortcut: 'h',
  run: () => {
    selection.moveColumn(-1);
  },
});

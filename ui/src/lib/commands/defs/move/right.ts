import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.right',
  menu: false,
  label: 'Next column',
  icon: 'board',
  group: 'Navigation',
  scope: 'board',
  shortcut: 'l',
  run: () => {
    selection.moveColumn(1);
  },
});

import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.up',
  menu: false,
  label: 'Move up',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'k',
  run: (_, n = 1) => {
    selection.move(selection.columns.flat(), -n);
  },
});

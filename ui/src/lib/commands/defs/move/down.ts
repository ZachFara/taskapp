import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.down',
  menu: false,
  label: 'Move down',
  icon: 'list',
  group: 'Navigation',
  shortcut: ['j', 'arrowdown'],
  run: (_, n = 1) => {
    selection.move(selection.columns.flat(), n);
  },
});

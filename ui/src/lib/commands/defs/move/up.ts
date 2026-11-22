import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.up',
  label: 'Move up',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'k',
  run: () => {
    selection.move(selection.columns.flat(), -1);
  },
});

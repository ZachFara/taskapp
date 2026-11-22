import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.down',
  label: 'Move down',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'j',
  run: () => {
    selection.move(selection.columns.flat(), 1);
  },
});

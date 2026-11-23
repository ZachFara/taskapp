import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.top',
  menu: false,
  label: 'Go to top',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'g g',
  run: () => {
    selection.focus(selection.columns.flat()[0] ?? null);
  },
});

import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.bottom',
  menu: false,
  label: 'Go to bottom',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'shift+g',
  run: () => {
    selection.focus(selection.columns.flat().at(-1) ?? null);
  },
});

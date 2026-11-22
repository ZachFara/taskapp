import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.bottom',
  label: 'Go to bottom',
  icon: 'list',
  group: 'Navigation',
  shortcut: 'G',
  run: () => {
    selection.focus(selection.columns.flat().at(-1) ?? null);
  },
});

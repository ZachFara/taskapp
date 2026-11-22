import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.right',
  label: 'Next column',
  icon: 'board',
  group: 'Navigation',
  shortcut: 'l',
  run: () => {
    selection.moveColumn(1);
  },
});

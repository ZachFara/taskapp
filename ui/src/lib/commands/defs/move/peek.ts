import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'move.peek',
  menu: false,
  label: 'Peek at issue',
  icon: 'eye',
  group: 'Navigation',
  shortcut: 'space',
  run: () => {
    selection.togglePeek();
  },
});

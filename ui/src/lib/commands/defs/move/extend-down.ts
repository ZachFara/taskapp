import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.extendDown',
  label: 'Extend selection down',
  icon: 'check',
  group: 'Selection',
  shortcut: 'shift+j',
  run: () => {
    selection.move(selection.columns.flat(), 1, true);
  },
});

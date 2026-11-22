import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.extendDown',
  menu: false,
  label: 'Extend selection down',
  icon: 'check',
  group: 'Selection',
  shortcut: 'shift+j',
  run: (_, n = 1) => {
    selection.move(selection.columns.flat(), n, true);
  },
});

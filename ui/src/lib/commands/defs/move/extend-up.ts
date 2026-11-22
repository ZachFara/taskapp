import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.extendUp',
  menu: false,
  label: 'Extend selection up',
  icon: 'check',
  group: 'Selection',
  shortcut: 'shift+k',
  run: (_, n = 1) => {
    selection.move(selection.columns.flat(), -n, true);
  },
});

import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.extendUp',
  label: 'Extend selection up',
  icon: 'check',
  group: 'Selection',
  shortcut: 'shift+k',
  run: () => {
    selection.move(selection.columns.flat(), -1, true);
  },
});

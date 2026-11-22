import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.all',
  menu: false,
  label: 'Select all in view',
  icon: 'check',
  group: 'Selection',
  shortcut: 'mod+a',
  run: () => {
    selection.selectAll(selection.columns.flat());
  },
});

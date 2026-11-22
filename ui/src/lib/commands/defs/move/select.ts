import { defineCommand } from '../../registry';

import { selection } from '../../../selection.svelte';

defineCommand({
  id: 'select.toggle',
  label: 'Select issue',
  icon: 'check',
  group: 'Selection',
  shortcut: 'x',
  run: () => {
    selection.toggle();
  },
});

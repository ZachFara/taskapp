import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the sidebar and the cycles page pass the cycle as the target.
defineCommand({
  id: 'cycle.delete',
  label: 'Delete cycle',
  icon: 'trash',
  group: 'Cycle',
  noShortcutReason: 'Reached from the row actions of a cycle only.',
  menu: false,
  run: (_c, _n, t) => t && askDelete(t as Target),
});

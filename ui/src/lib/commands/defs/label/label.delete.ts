import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the labels settings pass the label as the target.
defineCommand({
  id: 'label.delete',
  label: 'Delete label',
  icon: 'trash',
  group: 'Label',
  noShortcutReason: 'Reached from the row actions of a label only.',
  menu: false,
  run: (_c, _n, t) => t && askDelete(t as Target),
});

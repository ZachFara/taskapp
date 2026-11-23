import { defineCommand } from '../../registry';
import { askRename } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the labels settings pass the label as the target.
defineCommand({
  id: 'label.rename',
  label: 'Rename label',
  icon: 'edit',
  group: 'Label',
  noShortcutReason: 'Reached from the row actions of a label only.',
  menu: false,
  run: (_c, _n, t) => t && askRename(t as Target),
});

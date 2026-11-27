import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the templates settings pass the template as the target.
defineCommand({
  id: 'template.delete',
  label: 'Delete template',
  icon: 'trash',
  group: 'Template',
  noShortcutReason: 'Reached from the row actions of a template only.',
  menu: false,
  run: (_c, _n, t) => t && askDelete(t as Target),
});

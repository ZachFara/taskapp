import { defineCommand } from '../../registry';
import { askRename } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the templates settings pass the template as the target.
defineCommand({
  id: 'template.rename',
  label: 'Rename template',
  icon: 'edit',
  group: 'Template',
  noShortcutReason: 'Reached from the row actions of a template only.',
  menu: false,
  run: (_c, _n, t) => t && askRename(t as Target),
});

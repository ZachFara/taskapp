import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';
import type { Target } from '../../../rowActions';

// Row-only: the sidebar, the projects page and the settings screens pass the project as the target.
defineCommand({
  id: 'project.delete',
  label: 'Delete project',
  icon: 'trash',
  group: 'Project',
  noShortcutReason: 'Reached from the row actions of a project only.',
  menu: false,
  run: (_c, _n, t) => t && askDelete(t as Target),
});

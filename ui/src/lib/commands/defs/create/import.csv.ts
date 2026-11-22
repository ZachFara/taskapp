import { defineCommand } from '../../registry';
import { openForm } from '../_open';

// The page listens for `command:import.csv` and opens its form.
defineCommand({
  id: 'import.csv', label: 'Import CSV', icon: 'download', group: 'Create', noShortcutReason: 'a rare task, reached from Settings and the command menu',
  run: (ctx) => openForm(ctx, '/settings/import', 'command:import.csv'),
});

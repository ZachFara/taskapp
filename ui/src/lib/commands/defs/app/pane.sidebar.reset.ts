import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.sidebar.reset',
  label: 'Reset sidebar width',
  icon: 'undo',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the command menu; a double-click on the edge does the same.',
  run: emit('pane.sidebar.reset'),
});

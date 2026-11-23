import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.panel.reset',
  label: 'Reset issue panel width',
  icon: 'undo',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the command menu; a double-click on the edge does the same.',
  run: emit('pane.panel.reset'),
});

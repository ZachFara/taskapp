import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.panel.wider',
  label: 'Make issue panel wider',
  icon: 'panel',
  group: 'View',
  shortcut: 'alt+shift+arrowleft',
  run: emit('pane.panel.wider'),
});

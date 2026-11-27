import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.sidebar.wider',
  label: 'Make sidebar wider',
  icon: 'panel',
  group: 'View',
  shortcut: 'alt+arrowright',
  run: emit('pane.sidebar.wider'),
});

import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.panel.narrower',
  label: 'Make issue panel narrower',
  icon: 'panel',
  group: 'View',
  shortcut: 'alt+shift+arrowright',
  run: emit('pane.panel.narrower'),
});

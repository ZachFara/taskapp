import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'pane.sidebar.narrower',
  label: 'Make sidebar narrower',
  icon: 'panel',
  group: 'View',
  shortcut: 'alt+arrowleft',
  run: emit('pane.sidebar.narrower'),
});

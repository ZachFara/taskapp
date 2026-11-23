import { defineCommand } from '../../registry';
import { emit } from '../_event';

// Shell owns the sidebar; the registry only supplies the key and the menu entry.
defineCommand({
  id: 'sidebar.toggle',
  label: 'Show or hide sidebar',
  icon: 'panel',
  group: 'View',
  shortcut: 'mod+\\',
  run: emit('sidebar.toggle'),
});

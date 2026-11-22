import { defineCommand } from '../../registry';
import { emit } from '../_event';

// Global so it saves from inside a text field, where a settings page spends its time.
defineCommand({
  id: 'settings.save',
  label: 'Save settings',
  icon: 'check',
  group: 'Settings',
  shortcut: 'mod+s',
  global: true,
  when: (ctx) => ctx.route.startsWith('/settings'),
  run: emit('settings.save'),
});

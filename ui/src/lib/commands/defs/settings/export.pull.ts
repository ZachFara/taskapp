import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'settings.export.pull',
  label: 'Pull and apply from remote',
  icon: 'download',
  group: 'Settings',
  noShortcutReason: 'Reached from the command menu or the Export page',
  when: (ctx) => ctx.route === '/settings/export',
  run: emit('settings.export.pull'),
});

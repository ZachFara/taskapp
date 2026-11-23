import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'settings.restore',
  label: 'Restore from export…',
  icon: 'download',
  group: 'Settings',
  noShortcutReason: 'Rare and destructive; reached from the command menu',
  keywords: ['import', 'load', 'git', 'backup'],
  run: emit('settings.restore'),
});

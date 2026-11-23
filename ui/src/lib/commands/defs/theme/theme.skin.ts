import { defineCommand } from '../../registry';
import { SKINS, setSkin, theme } from '../../../theme.svelte';

defineCommand({
  id: 'theme.skin',
  label: 'Change theme…',
  icon: 'sun',
  group: 'Help',
  noShortcutReason: 'a rare switch, reached from Settings and the command menu',
  keywords: ['skin', 'appearance', 'look', 'colors'],
  page: () => ({
    title: 'Change theme',
    items: SKINS.map((s) => ({ id: s.id, label: s.label, icon: 'sun', checked: () => theme.skin === s.id, run: () => setSkin(s.id) })),
  }),
});

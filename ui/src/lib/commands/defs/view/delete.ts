import { defineCommand } from '../../registry';
import { askDelete } from '../../../rowUi.svelte';

defineCommand({
  id: 'view.delete',
  label: 'Delete view',
  icon: 'trash',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: (_c, _n, t) => (t ? askDelete(t as Target) : dispatchEvent(new CustomEvent('command:view.delete'))),
});

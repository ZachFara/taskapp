import { defineCommand } from '../../registry';
import { togglePin, type Target } from '../../../rowActions';

defineCommand({
  id: 'view.pin',
  label: 'Pin or unpin view',
  icon: 'pin',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu and the command menu.',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: (_c, _n, t) => (t ? togglePin(t as Target) : dispatchEvent(new CustomEvent('command:view.pin'))),
});

import { defineCommand } from '../../registry';
import { copyLink } from '../../../deepLink';
import type { Target } from '../../../rowActions';
import { onView } from '../_event';

defineCommand({
  id: 'view.copyLink',
  label: 'Copy link to view',
  icon: 'link',
  group: 'View',
  noShortcutReason: 'Rare action, reached from the view menu, the right-click menu and the command menu.',
  when: onView,
  run: async (ctx, _n, t) => copyLink('view', (t as Target | undefined)?.id ?? ctx.route.split('/')[2]),
});

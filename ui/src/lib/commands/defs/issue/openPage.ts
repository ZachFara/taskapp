import { defineCommand } from '../../registry';
import { keyOf } from './_issue';

defineCommand({
  id: 'issue.openPage',
  label: 'Open full page',
  icon: 'arrow-right',
  group: 'Issue',
  shortcut: 'mod+enter',
  when: (ctx) => !ctx.route.startsWith('/issue/'),
  run: async (ctx) => ctx.navigate(`/issue/${await keyOf(ctx)}`),
});

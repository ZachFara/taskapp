import { defineCommand } from '../../registry';
import { pageOf, patchOf } from './_issue';

const names = [['No priority', 0], ['Urgent', 4], ['High', 3], ['Medium', 2], ['Low', 1]] as const;

defineCommand({
  id: 'issue.priority', label: 'Set priority', icon: 'flag', group: 'Issue', shortcut: 'p',
  page: (ctx) => pageOf(ctx, 'Priority', names.map(([n, p]) => [n, patchOf({ priority: p })])),
});

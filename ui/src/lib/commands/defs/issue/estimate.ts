import { defineCommand } from '../../registry';
import { pageOf, patchOf } from './_issue';

const sizes: [string, number | null][] = [['15 minutes', 15], ['30 minutes', 30], ['1 hour', 60], ['2 hours', 120], ['4 hours', 240], ['1 day', 480], ['No estimate', null]];

defineCommand({
  id: 'issue.estimate', label: 'Set estimate', icon: 'clock', group: 'Issue', shortcut: 'e',
  page: (ctx) => pageOf(ctx, 'Estimate', sizes.map(([n, m]) => [n, patchOf({ estimate_minutes: m })])),
});

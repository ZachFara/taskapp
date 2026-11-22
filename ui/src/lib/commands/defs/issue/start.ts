import { defineCommand } from '../../registry';
import { pageOf, patchOf, today } from './_issue';

defineCommand({
  id: 'issue.start', label: 'Set start date', icon: 'calendar', group: 'Issue', shortcut: 'shift+d',
  page: (ctx) =>
    pageOf(ctx, 'Start date', [
      ['Today', patchOf({ start_date: today() })],
      ['Tomorrow', patchOf({ start_date: today(1) })],
      ['In a week', patchOf({ start_date: today(7) })],
      ['No start date', patchOf({ start_date: null })],
    ]),
});

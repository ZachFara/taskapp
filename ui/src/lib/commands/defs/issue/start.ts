import { defineCommand } from '../../registry';
import { dateParse, day, pageOf, patchOf } from './_issue';

defineCommand({
  id: 'issue.start', label: 'Set start date', icon: 'calendar', group: 'Issue', shortcut: 'shift+d',
  page: (ctx) =>
    pageOf(ctx, 'Start date', () => [
      ['Today', patchOf({ start_date: day() })],
      ['Tomorrow', patchOf({ start_date: day(1) })],
      ['In a week', patchOf({ start_date: day(7) })],
      ['No start date', patchOf({ start_date: null })],
    ], { parse: dateParse(ctx, 'start_date') }),
});

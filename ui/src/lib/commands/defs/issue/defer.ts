import { defineCommand } from '../../registry';
import { formatDay, parseDate } from '../../../parse/when';
import { apply, choices, day, pageOf, patchOf } from './_issue';

// The tickler: hide the issue until a day; it returns to the Inbox then. No due date, no calendar, no status change.
defineCommand({
  id: 'issue.defer', label: 'Defer (comes back on a day)', icon: 'clock', group: 'Issue',
  noShortcutReason: 'Reached from the Comes back row and Defer button of the issue page, the right-click menu and the command menu.',
  page: (ctx) =>
    pageOf(ctx, 'Comes back', () => [
      ['Tomorrow', patchOf({ hidden_until: day(1) })],
      ['In a week', patchOf({ hidden_until: day(7) })],
      ['In a month', patchOf({ hidden_until: day(30) })],
      ['Bring it back now', patchOf({ hidden_until: null })],
    ], {
      // Typed text such as `fri`, `nov 28`, `+3d`, or `none` to bring it back now.
      parse: (text) => {
        const r = parseDate(text, choices.tz);
        if (!r || (r.value !== null && r.value <= day())) return null;
        return {
          label: r.value === null ? 'Bring it back now' : `Comes back on ${formatDay(r.value)}`,
          run: () => apply(ctx, patchOf({ hidden_until: r.value })),
        };
      },
    }),
});

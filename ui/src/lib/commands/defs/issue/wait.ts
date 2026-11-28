import { defineCommand } from '../../registry';
import { formatDay, parseDate } from '../../../parse/when';
import { apply, choices, day, pageOf, patchOf } from './_issue';

// Who the issue waits on; "Stop waiting" clears the note and its follow-up day together.
defineCommand({
  id: 'issue.wait', label: 'Waiting on', icon: 'user', group: 'Issue',
  noShortcutReason: 'Reached from the Waiting on row of the issue details and the command menu.',
  page: (ctx) =>
    pageOf(ctx, 'Waiting on', [['Stop waiting', patchOf({ waiting_on: null, follow_up: null })]], {
      parse: (text) => {
        const who = text.trim();
        return who ? { label: `Waiting on ${who}`, run: () => apply(ctx, patchOf({ waiting_on: who })) } : null;
      },
    }),
});

// The day to check back; once it has come the issue shows as a follow-up due.
defineCommand({
  id: 'issue.followUp', label: 'Follow-up day', icon: 'calendar', group: 'Issue',
  noShortcutReason: 'Reached from the Follow up row of the issue details and the command menu.',
  page: (ctx) =>
    pageOf(ctx, 'Follow up', [
      ['Tomorrow', patchOf({ follow_up: day(1) })],
      ['In a week', patchOf({ follow_up: day(7) })],
      ['No follow-up day', patchOf({ follow_up: null })],
    ], {
      // Typed text such as `fri`, `nov 28`, `+3d`, or `none` to clear it.
      parse: (text) => {
        const r = parseDate(text, choices.tz);
        if (!r) return null;
        return {
          label: r.value === null ? 'Clear the follow-up day' : `Follow up on ${formatDay(r.value)}`,
          run: () => apply(ctx, patchOf({ follow_up: r.value })),
        };
      },
    }),
});

import { defineCommand } from '../../registry';
import { pageOf, patchOf, today } from './_issue';

// shortcut: presets instead of free typing (the command page has no text input), when pages get one.
defineCommand({
  id: 'issue.due', label: 'Set due date', icon: 'calendar', group: 'Issue', shortcut: 'd',
  page: (ctx) =>
    pageOf(ctx, 'Due date', [
      ['Today', patchOf({ due: { Date: today() } })],
      ['Tomorrow', patchOf({ due: { Date: today(1) } })],
      ['In a week', patchOf({ due: { Date: today(7) } })],
      ['In two weeks', patchOf({ due: { Date: today(14) } })],
      ['No due date', patchOf({ due: null })],
    ]),
});

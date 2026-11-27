import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { dateParse, day, pageOf, patchOf } from './_issue';

// Presets for the common cases; anything else is typed (`fri`, `nov 28`, `+3d`, `none`) and shown as a preview row.
defineCommand({
  id: 'issue.due', label: 'Set due date', icon: 'calendar', group: 'Issue', shortcut: 'd',
  when: () => fieldOn('due'),
  page: (ctx) =>
    pageOf(ctx, 'Due date', () => [
      ['Today', patchOf({ due: { Date: day() } })],
      ['Tomorrow', patchOf({ due: { Date: day(1) } })],
      ['In a week', patchOf({ due: { Date: day(7) } })],
      ['No due date', patchOf({ due: null })],
    ], { parse: dateParse(ctx, 'due') }),
});

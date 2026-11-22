import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf } from './_issue';

defineCommand({
  id: 'issue.cycle', label: 'Set cycle', icon: 'cycle', group: 'Issue', shortcut: 'shift+c',
  page: (ctx) =>
    pageOf(ctx, 'Cycle', [['No cycle', patchOf({ cycle_id: null })], ...choices.cycles.map((c): [string, ReturnType<typeof patchOf>] => [c.name ?? `Cycle ${c.number}`, patchOf({ cycle_id: c.id })])]),
});

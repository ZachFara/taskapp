import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { choices, pageOf, patchOf, type Row } from './_issue';

defineCommand({
  id: 'issue.cycle', label: 'Set cycle', icon: 'cycle', group: 'Issue', shortcut: 'shift+c',
  when: () => fieldOn('cycle'),
  page: (ctx) =>
    pageOf(ctx, 'Cycle', () => [['No cycle', patchOf({ cycle_id: null })], ...choices.cycles.map((c): Row => [c.name ?? `Cycle ${c.number}`, patchOf({ cycle_id: c.id }), { entity: { icon: c.icon, color: c.color } }])]),
});

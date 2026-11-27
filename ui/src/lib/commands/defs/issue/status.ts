import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf, type Row } from './_issue';

defineCommand({
  id: 'issue.status', label: 'Set status', icon: 'circle', group: 'Issue', shortcut: 's',
  page: (ctx) => pageOf(ctx, 'Status', () => choices.statuses.map((s): Row => [s.name, patchOf({ status_id: s.id }), { status: s }])),
});

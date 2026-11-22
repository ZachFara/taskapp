import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf } from './_issue';

defineCommand({
  id: 'issue.labels', label: 'Add or remove labels', icon: 'tag', group: 'Issue', shortcut: ['shift+l', 'l'],
  page: (ctx) =>
    pageOf(ctx, 'Labels', [
      ...choices.labels.map((l): [string, ReturnType<typeof patchOf>] => [`Add ${l.name}`, patchOf({}, { add_labels: [l.id] })]),
      ...choices.labels.map((l): [string, ReturnType<typeof patchOf>] => [`Remove ${l.name}`, patchOf({}, { remove_labels: [l.id] })]),
    ]),
});

import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { choices, pageOf, patchOf, type Row } from './_issue';

defineCommand({
  id: 'issue.labels', label: 'Add or remove labels', icon: 'tag', group: 'Issue', shortcut: ['shift+l', 'l'],
  when: () => fieldOn('labels'),
  page: (ctx) =>
    pageOf(ctx, 'Labels', () => [
      ...choices.labels.map((l): Row => [`Add ${l.name}`, patchOf({}, { add_labels: [l.id] }), { entity: { icon: l.icon ?? 'icon:tag', color: l.color } }]),
      ...choices.labels.map((l): Row => [`Remove ${l.name}`, patchOf({}, { remove_labels: [l.id] }), { entity: { icon: l.icon ?? 'icon:tag', color: l.color } }]),
    ]),
});

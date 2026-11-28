import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { pageOf, patchOf } from './_issue';

defineCommand({
  id: 'issue.depth', label: 'Set depth', icon: 'layers', group: 'Issue',
  noShortcutReason: 'Reached from the Depth row of the issue details and the command menu.',
  when: () => fieldOn('depth'),
  page: (ctx) => pageOf(ctx, 'Depth', [
    ['Deep work', patchOf({ depth: 'deep' })],
    ['Shallow work', patchOf({ depth: 'shallow' })],
    ['No depth', patchOf({ depth: null })],
  ]),
});

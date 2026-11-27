import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { estimateParse, pageOf, patchOf } from './_issue';

const sizes: [string, number | null][] = [['30 minutes', 30], ['1 hour', 60], ['2 hours', 120], ['1 day', 480], ['No estimate', null]];

defineCommand({
  id: 'issue.estimate', label: 'Set estimate', icon: 'clock', group: 'Issue', shortcut: 'e',
  when: () => fieldOn('estimate'),
  page: (ctx) => pageOf(ctx, 'Estimate', sizes.map(([n, m]) => [n, patchOf({ estimate_minutes: m })]), { parse: estimateParse(ctx) }),
});

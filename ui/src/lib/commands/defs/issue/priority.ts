import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { PRIORITY_OPTIONS } from '../../../priority';
import { pageOf, patchOf } from './_issue';


defineCommand({
  id: 'issue.priority', label: 'Set priority', icon: 'flag', group: 'Issue', shortcut: 'p',
  when: () => fieldOn('priority'),
  page: (ctx) => pageOf(ctx, 'Priority', PRIORITY_OPTIONS.map(({ name, value }) => [name, patchOf({ priority: value })])),
});

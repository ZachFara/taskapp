import { defineCommand } from '../../registry';
import { fieldOn } from '../../../fields.svelte';
import { choices, pageOf, patchOf } from './_issue';

defineCommand({
  id: 'issue.project', label: 'Move to project', icon: 'folder', group: 'Issue', shortcut: 'm',
  when: () => fieldOn('project'),
  page: (ctx) =>
    pageOf(ctx, 'Project', () => [['No project', patchOf({ project_id: null })], ...choices.projects.map((p): [string, ReturnType<typeof patchOf>] => [p.name, patchOf({ project_id: p.id })])]),
});

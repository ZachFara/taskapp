import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf, targets } from './_issue';

defineCommand({
  id: 'issue.parent', label: 'Set parent', icon: 'sub-issue', group: 'Issue', shortcut: 'shift+p',
  page: (ctx) => {
    const own = targets(ctx);
    return pageOf(ctx, 'Parent', [
      ['No parent', patchOf({ parent_id: null })],
      ...choices.issues.filter((i) => !own.includes(i.id)).map((i): [string, ReturnType<typeof patchOf>] => [`${i.key} ${i.title}`, patchOf({ parent_id: i.id })]),
    ]);
  },
});

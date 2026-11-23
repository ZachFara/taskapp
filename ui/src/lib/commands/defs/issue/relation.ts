import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf, targets } from './_issue';

defineCommand({
  id: 'issue.relation', label: 'Add relation', icon: 'link', group: 'Issue', shortcut: 'r',
  page: (ctx) => {
    const own = targets(ctx);
    return pageOf(ctx, 'Relation', () => choices.relationKinds.flatMap(({ name, id: kind }) =>
      choices.issues.filter((i) => !own.includes(i.id)).map((i): [string, ReturnType<typeof patchOf>] => [`${name} ${i.key} ${i.title}`, patchOf({}, { add_relation: [kind, i.id] })])));
  },
});

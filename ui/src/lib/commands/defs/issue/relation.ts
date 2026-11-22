import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf, targets } from './_issue';
import type { RelationKind } from '../../../bindings';

const kinds: [string, RelationKind][] = [['Blocks', 'Blocks'], ['Related to', 'Related'], ['Duplicate of', 'Duplicate']];

defineCommand({
  id: 'issue.relation', label: 'Add relation', icon: 'link', group: 'Issue', shortcut: 'r',
  page: (ctx) => {
    const own = targets(ctx);
    const others = choices.issues.filter((i) => !own.includes(i.id));
    return pageOf(ctx, 'Relation', kinds.flatMap(([name, kind]) =>
      others.map((i): [string, ReturnType<typeof patchOf>] => [`${name} ${i.key} ${i.title}`, patchOf({}, { add_relation: [kind, i.id] })])));
  },
});

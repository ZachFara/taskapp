import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf, targets } from './_issue';
import type { RelationKind } from '../../../bindings';

const kinds: [string, RelationKind][] = [['Blocks', 'Blocks'], ['Related to', 'Related'], ['Duplicate of', 'Duplicate']];

defineCommand({
  id: 'issue.relation', label: 'Add relation', icon: 'link', group: 'Issue', shortcut: 'r',
  page: (ctx) => {
    const own = targets(ctx);
    return pageOf(ctx, 'Relation', () => kinds.flatMap(([name, kind]) =>
      choices.issues.filter((i) => !own.includes(i.id)).map((i): [string, ReturnType<typeof patchOf>] => [`${name} ${i.key} ${i.title}`, patchOf({}, { add_relation: [kind, i.id] })])));
  },
});

import { defineCommand } from '../../registry';
import { archive, CONFIRM_ABOVE, apply, patchOf, targets } from './_issue';

defineCommand({ id: 'issue.archive', label: 'Archive', icon: 'archive', group: 'Issue', shortcut: '#', run: archive });

// The confirm step for a big selection; `archive` opens it. It only lists while there is a big selection to confirm.
defineCommand({
  id: 'issue.archive.confirm', label: 'Archive the selected issues', icon: 'archive', group: 'Issue',
  noShortcutReason: 'the confirm step of Archive, opened by it',
  when: (ctx) => targets(ctx).length > CONFIRM_ABOVE,
  page: (ctx) => {
    const n = targets(ctx).length;
    return {
      title: `Archive ${n} issues?`,
      items: [
        { id: 'archive', label: `Archive ${n} issues`, icon: 'archive', run: () => apply(ctx, patchOf({}, { archive: true })) },
        { id: 'keep', label: 'Keep them', icon: 'close', run: () => {} },
      ],
    };
  },
});

import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.parent',
  label: 'Set parent issue',
  icon: 'sub-issue',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'rare edit, reached from the properties list or the command menu',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'parent' })),
});

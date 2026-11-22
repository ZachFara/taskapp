import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.copyKey',
  label: 'Copy issue key',
  icon: 'copy',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'the key in the header copies on click',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.copyKey')),
});

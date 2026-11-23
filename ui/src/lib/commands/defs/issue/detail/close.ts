import { defineCommand } from '../../../registry';

// Esc closes an open editor first, then the panel; the screen decides which, the registry only delivers the key.
defineCommand({
  id: 'issue.detail.close',
  menu: false,
  label: 'Close issue',
  icon: 'close',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'escape',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.close')),
});

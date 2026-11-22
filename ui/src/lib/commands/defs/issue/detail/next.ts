import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.next',
  label: 'Next issue',
  icon: 'arrow-down',
  group: 'Issue',
  scope: 'detail',
  shortcut: ']',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.next')),
});

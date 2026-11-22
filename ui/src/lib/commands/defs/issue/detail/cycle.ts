import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.cycle',
  label: 'Change cycle',
  icon: 'cycle',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'shift+c',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'cycle' })),
});

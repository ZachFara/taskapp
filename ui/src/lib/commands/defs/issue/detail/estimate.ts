import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.estimate',
  label: 'Set estimate',
  icon: 'clock',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'e',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'estimate' })),
});

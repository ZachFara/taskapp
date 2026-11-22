import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.labels',
  label: 'Change labels',
  icon: 'tag',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'l',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'labels' })),
});

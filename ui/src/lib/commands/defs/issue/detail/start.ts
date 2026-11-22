import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.start',
  label: 'Set start date',
  icon: 'calendar',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'shift+d',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'start' })),
});

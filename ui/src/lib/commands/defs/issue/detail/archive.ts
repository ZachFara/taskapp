import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.archive',
  label: 'Archive or restore issue',
  icon: 'archive',
  group: 'Issue',
  shortcut: 'shift+a',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.archive')),
});

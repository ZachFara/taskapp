import { defineCommand } from '../../../registry';

// The open issue screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.detail.pin',
  label: 'Pin or unpin issue',
  icon: 'pin',
  group: 'Issue',
  scope: 'detail',
  shortcut: 'alt+p',
  when: (c) => c.route.startsWith('/issue/') || c.openIssueId !== null,
  run: () => dispatchEvent(new CustomEvent('command:issue.detail.pin')),
});

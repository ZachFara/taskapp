import { defineCommand } from '../../../registry';
import { emit, onIssue } from '../../_event';

// The Links section of the open issue focuses its URL field when it hears this.
defineCommand({
  id: 'issue.link.add',
  label: 'Add a link to this issue',
  icon: 'link',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'Rare action, reached from the Links section and the command menu.',
  when: onIssue,
  run: emit('issue.link.add'),
});

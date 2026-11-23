import { defineCommand } from '../../../registry';
import { emit, onIssue } from '../../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'issue.description.edit',
  label: 'Edit description',
  icon: 'edit',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'Reached from the issue panel and the command menu.',
  when: onIssue,
  run: emit('issue.description.edit'),
});

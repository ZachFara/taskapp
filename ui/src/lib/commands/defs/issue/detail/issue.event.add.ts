import { defineCommand } from '../../../registry';
import { emit, onIssue } from '../../_event';

// The panel's Events section owns this control and listens for the event.
defineCommand({
  id: 'issue.event.add',
  label: 'Add issue to an event',
  icon: 'calendar',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'Reached from the Events section of the issue and the command menu.',
  when: onIssue,
  run: emit('issue.event.add'),
});

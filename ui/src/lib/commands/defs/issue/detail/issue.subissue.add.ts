import { defineCommand } from '../../../registry';
import { emit, onIssue } from '../../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'issue.subissue.add',
  label: 'Add sub-issue',
  icon: 'sub-issue',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'Reached from the issue panel and the command menu.',
  when: onIssue,
  run: emit('issue.subissue.add'),
});

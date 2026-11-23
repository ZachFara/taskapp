import { defineCommand } from '../../../registry';
import { emit, onIssue } from '../../_event';

// The panel's relation control calls the same handler through this event. The menu already offers `issue.relation`
// (key r), which is the same action, so this one stays out of the menu rather than list it twice.
defineCommand({
  id: 'issue.relation.add',
  menu: false,
  label: 'Add relation from the panel',
  icon: 'link',
  group: 'Issue',
  scope: 'detail',
  noShortcutReason: 'The key r (issue.relation) does the same.',
  when: onIssue,
  run: emit('issue.relation.add'),
});

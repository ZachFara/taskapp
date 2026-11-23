import { defineCommand } from '../../registry';
import { emit, onProject } from '../_event';

defineCommand({
  id: 'milestone.new',
  label: 'New milestone',
  icon: 'target',
  group: 'Create',
  noShortcutReason: 'A button on the project page and the command menu.',
  when: onProject,
  run: emit('milestone.new'),
});

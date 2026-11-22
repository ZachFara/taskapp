import { defineCommand } from '../../registry';
import { emit, onProject } from '../_event';

// The screen that owns this control listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'project.description.edit',
  label: 'Edit project description',
  icon: 'edit',
  group: 'Project',
  noShortcutReason: 'Rare action, reached from the project page and the command menu.',
  when: onProject,
  run: emit('project.description.edit'),
});

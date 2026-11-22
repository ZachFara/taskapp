import { defineCommand } from '../../registry';

// The project page owns the pin state and listens for this event; the registry only decides when it is offered.
defineCommand({
  id: 'project.pin',
  label: 'Pin or unpin project',
  icon: 'pin',
  group: 'Project',
  noShortcutReason: 'Occasional action; reachable from the command menu and the page header.',
  when: (ctx) => ctx.route.startsWith('/project/'),
  run: () => dispatchEvent(new CustomEvent('command:project.pin')),
});

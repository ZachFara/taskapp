import { defineCommand } from '../../registry';

// Opens the confirmation on the project page; archiving is never one keystroke.
defineCommand({
  id: 'project.archive',
  label: 'Archive project',
  icon: 'archive',
  group: 'Project',
  noShortcutReason: 'Destructive; always goes through a confirmation, so no key.',
  when: (ctx) => ctx.route.startsWith('/project/'),
  run: () => dispatchEvent(new CustomEvent('command:project.archive')),
});

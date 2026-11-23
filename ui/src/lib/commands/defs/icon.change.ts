import { defineCommand } from '../registry';
import { onProject, onView } from './_event';

// The page owns its icon picker and listens for the event; the registry only decides when it is offered.
defineCommand({
  id: 'icon.change',
  label: 'Change icon',
  icon: 'circle',
  group: 'View',
  noShortcutReason: 'Reached from the page header and the command menu.',
  when: (ctx) => onView(ctx) || onProject(ctx),
  run: () => dispatchEvent(new CustomEvent('command:icon.change')),
});

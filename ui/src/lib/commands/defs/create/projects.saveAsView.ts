import { defineCommand } from '../../registry';

// The Projects page listens for `command:projects.saveAsView` and saves what it shows as a project view.
defineCommand({
  id: 'projects.saveAsView', label: 'Save projects as view', icon: 'layers', group: 'Create', noShortcutReason: 'a button on the Projects page and the command menu',
  when: (ctx) => ctx.route === '/projects',
  run: () => dispatchEvent(new CustomEvent('command:projects.saveAsView')),
});

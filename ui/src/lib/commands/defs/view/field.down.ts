import { defineCommand } from '../../registry';

// The Display panel moves the field whose row has focus; the registry only decides when the key fires.
defineCommand({
  id: 'view.field.down',
  label: 'Move field down',
  icon: 'chevron-right',
  group: 'View',
  shortcut: 'alt+arrowdown',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:view.field.down')),
});

import { defineCommand } from '../../registry';

// The table screen listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'table.cancel',
  label: 'Cancel edit or clear selection',
  icon: 'close',
  group: 'Table',
  shortcut: 'escape',
  when: (ctx) => ctx.route.startsWith('/view/'),
  run: () => dispatchEvent(new CustomEvent('command:table.cancel')),
});

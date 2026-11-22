import { defineCommand } from '../registry';

// The screen that owns this action listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'help.shortcuts',
  label: 'Keyboard shortcuts',
  icon: 'help',
  group: 'Help',
  shortcut: '?',
  run: () => dispatchEvent(new CustomEvent('command:help.shortcuts')),
});

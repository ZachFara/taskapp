import { defineCommand } from '../registry';

// The screen that owns this action listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'palette.open',
  label: 'Open command menu',
  icon: 'search',
  group: 'Navigate',
  shortcut: ['mod+k', 'mod+p'],
  global: true,
  run: () => dispatchEvent(new CustomEvent('command:palette.open')),
});

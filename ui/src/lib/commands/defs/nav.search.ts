import { defineCommand } from '../registry';

// The screen that owns this action listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'nav.search',
  label: 'Search issues…',
  icon: 'search',
  group: 'Navigation',
  shortcut: '/',
  run: () => dispatchEvent(new CustomEvent('command:nav.search')),
});

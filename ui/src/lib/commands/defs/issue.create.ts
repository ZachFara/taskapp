import { defineCommand } from '../registry';

// The screen that owns this action listens for the event; the registry only decides when it fires.
defineCommand({
  id: 'issue.create',
  label: 'New issue',
  icon: 'plus',
  group: 'Create',
  shortcut: 'c',
  run: () => dispatchEvent(new CustomEvent('command:issue.create')),
});

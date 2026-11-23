import { defineCommand } from '../../registry';

// The timeline screen listens for the event; it is offered only while a timeline is on screen.
defineCommand({
  id: 'timeline.today',
  label: 'Timeline: go to today',
  icon: 'target',
  group: 'Timeline',
  scope: 'timeline',
  shortcut: 't',
  when: () => !!document.querySelector('[data-timeline]'),
  run: () => dispatchEvent(new CustomEvent('command:timeline.today')),
});

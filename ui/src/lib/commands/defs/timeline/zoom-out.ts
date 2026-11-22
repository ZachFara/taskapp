import { defineCommand } from '../../registry';

// The timeline screen listens for the event; it is offered only while a timeline is on screen.
defineCommand({
  id: 'timeline.zoom-out',
  label: 'Timeline: zoom out',
  icon: 'layers',
  group: 'Timeline',
  scope: 'timeline',
  shortcut: '-',
  when: () => !!document.querySelector('[data-timeline]'),
  run: () => dispatchEvent(new CustomEvent('command:timeline.zoom-out')),
});

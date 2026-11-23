import { defineCommand } from '../registry';
import { onProject, onView } from './_event';
import { rowUi, pickKey } from '../../rowUi.svelte';
import type { Target } from '../../rowActions';

// A page owns its header picker and listens for the event. A row passes its target and opens that row's picker.
defineCommand({
  id: 'icon.change',
  label: 'Change icon',
  icon: 'circle',
  group: 'View',
  noShortcutReason: 'Reached from the page header and the row actions.',
  when: (ctx) => onView(ctx) || onProject(ctx),
  run: (_c, _n, t) => (t ? (rowUi.picking = pickKey(t as Target)) : dispatchEvent(new CustomEvent('command:icon.change'))),
});

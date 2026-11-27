import { defineCommand } from '../../registry';
import { setTooltips, tooltips } from '../../../tooltips.svelte';

defineCommand({
  id: 'tooltips.toggle',
  label: 'Show or hide tooltips',
  icon: 'help',
  group: 'Settings',
  noShortcutReason: 'Rare setting; the Settings page and the command menu cover it.',
  run: () => setTooltips(!tooltips.show),
});

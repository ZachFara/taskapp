import { defineCommand } from '../../registry';
import { stepSpeed } from '../../../motion.svelte';

defineCommand({
  id: 'motion.slower',
  label: 'Slower animations',
  icon: 'plus',
  group: 'Settings',
  noShortcutReason: 'Rare setting; the Settings page and the command menu cover it.',
  run: () => stepSpeed(1),
});

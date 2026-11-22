import { defineCommand } from '../../registry';
import { cycleTheme } from '../../../theme.svelte';

defineCommand({
  id: 'theme.cycle',
  label: 'Switch theme (light, dark, system)',
  icon: 'sun',
  group: 'Help',
  noShortcutReason: 'a rare switch, reached from the top bar and the command menu',
  run: cycleTheme,
});

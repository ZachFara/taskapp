import { defineCommand } from '../../registry';
import { saveTomorrow, shutdown } from '../../../shutdown.svelte';

defineCommand({
  id: 'shutdown.saveTomorrow', label: "Save tomorrow's picks", icon: 'flag', group: 'Shutdown',
  noShortcutReason: 'A button in the shutdown checklist.',
  when: () => shutdown.open,
  run: () => saveTomorrow(),
});

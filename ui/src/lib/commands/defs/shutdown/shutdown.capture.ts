import { defineCommand } from '../../registry';
import { captureLooseEnd, shutdown } from '../../../shutdown.svelte';

defineCommand({
  id: 'shutdown.capture', label: 'Capture loose end', icon: 'plus', group: 'Shutdown',
  noShortcutReason: 'A button in the shutdown checklist.',
  when: () => shutdown.open,
  run: () => captureLooseEnd(),
});

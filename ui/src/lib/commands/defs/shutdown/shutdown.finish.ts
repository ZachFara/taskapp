import { defineCommand } from '../../registry';
import { finishShutdown, shutdown } from '../../../shutdown.svelte';

defineCommand({
  id: 'shutdown.finish', label: 'Finish shutdown', icon: 'check', group: 'Shutdown',
  noShortcutReason: 'A button in the shutdown checklist.',
  when: () => shutdown.open,
  run: () => finishShutdown(),
});

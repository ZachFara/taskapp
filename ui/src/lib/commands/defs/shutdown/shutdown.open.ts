import { defineCommand } from '../../registry';
import { openShutdown } from '../../../shutdown.svelte';

defineCommand({
  id: 'shutdown.open', label: 'Shutdown', icon: 'check', group: 'Actions',
  noShortcutReason: 'Opens the end-of-day checklist from the command menu or the My issues page.',
  run: () => openShutdown(),
});

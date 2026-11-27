import { dashboard, set } from '../../../../components/dashboard/dashboard.svelte';
import { defineCommand } from '../../registry';

defineCommand({
  id: 'dashboard.measure',
  label: 'Dashboard: count or points',
  icon: 'bolt',
  group: 'View',
  noShortcutReason: 'reached from the Count and Points switch and the command menu',
  when: (ctx) => dashboard.on && ctx.route === '/dashboard',
  run: () => set({ measure: dashboard.measure === 'Count' ? 'Points' : 'Count' }),
});

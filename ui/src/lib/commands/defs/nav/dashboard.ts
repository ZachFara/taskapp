import { dashboard } from '../../../../components/dashboard/dashboard.svelte';
import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.dashboard',
  label: 'Go to Dashboard',
  icon: 'chart',
  group: 'Navigation',
  shortcut: 'g b',
  when: () => dashboard.on,
  run: (ctx) => ctx.navigate('/dashboard'),
});

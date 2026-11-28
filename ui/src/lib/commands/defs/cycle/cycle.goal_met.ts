import { defineCommand } from '../../registry';
import { setRetro } from './_retro';

defineCommand({
  id: 'cycle.goal_met',
  label: 'Was the cycle goal met…',
  icon: 'check',
  group: 'Cycle',
  noShortcutReason: 'reached from the command menu and the retrospective switch',
  keywords: ['retrospective', 'goal', 'yes', 'no'],
  page: (ctx) => ({
    title: 'Goal met',
    items: [
      { id: 'yes', label: 'Yes', run: () => setRetro(ctx, { goal_met: true }) },
      { id: 'no', label: 'No', run: () => setRetro(ctx, { goal_met: false }) },
      { id: 'unset', label: 'Not decided', run: () => setRetro(ctx, { goal_met: null }) },
    ],
  }),
});

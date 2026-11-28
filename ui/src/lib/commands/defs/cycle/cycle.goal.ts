import { defineCommand } from '../../registry';
import { setRetro } from './_retro';

defineCommand({
  id: 'cycle.goal',
  label: 'Set cycle goal…',
  icon: 'cycle',
  group: 'Cycle',
  noShortcutReason: 'reached from the command menu and the retrospective field',
  keywords: ['retrospective', 'goal', 'planning'],
  page: (ctx) => ({
    title: 'Cycle goal',
    // The goal is typed into the palette as one sentence.
    parse: (text) => (text.trim() ? { label: `Goal: ${text.trim()}`, run: () => setRetro(ctx, { goal: text.trim() }) } : null),
    items: [{ id: 'clear', label: 'Clear the goal', run: () => setRetro(ctx, { goal: null }) }],
  }),
});

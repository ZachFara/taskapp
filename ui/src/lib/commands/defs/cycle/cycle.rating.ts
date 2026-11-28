import { defineCommand } from '../../registry';
import { setRetro } from './_retro';

defineCommand({
  id: 'cycle.rating',
  label: 'Rate the cycle…',
  icon: 'star',
  group: 'Cycle',
  noShortcutReason: 'reached from the command menu and the retrospective rating',
  keywords: ['retrospective', 'rating', 'how it went'],
  page: (ctx) => ({
    title: 'How did the cycle go',
    items: [
      ...[1, 2, 3, 4, 5].map((n) => ({ id: String(n), label: `${n} of 5`, run: () => setRetro(ctx, { rating: n }) })),
      { id: 'unset', label: 'Not rated', run: () => setRetro(ctx, { rating: null }) },
    ],
  }),
});

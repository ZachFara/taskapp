import { CARDS } from '../../../../components/dashboard/cards';
import { dashboard, moveCard, orderedCards, toggleCard } from '../../../../components/dashboard/dashboard.svelte';
import { defineCommand } from '../../registry';

const title = (id: string) => CARDS.find((c) => c.id === id)!.title;

defineCommand({
  id: 'dashboard.cards',
  label: 'Dashboard: show or hide cards…',
  icon: 'eye',
  group: 'View',
  noShortcutReason: 'reached from the Cards button and the command menu',
  when: (ctx) => dashboard.on && ctx.route === '/dashboard',
  page: () => ({
    title: 'Dashboard cards',
    items: CARDS.map((c) => ({ id: c.id, label: c.title, icon: 'chart', checked: () => !dashboard.hidden.includes(c.id), run: () => toggleCard(c.id) })),
  }),
});

defineCommand({
  id: 'dashboard.cards.move',
  label: 'Dashboard: move a card…',
  icon: 'arrow-up',
  group: 'View',
  noShortcutReason: 'reached from the Cards button and the command menu',
  when: (ctx) => dashboard.on && ctx.route === '/dashboard',
  page: () => ({
    title: 'Move a dashboard card',
    items: orderedCards().flatMap((id) => [
      { id: `${id}:up`, label: `Move ${title(id)} earlier`, icon: 'arrow-up', run: () => moveCard(id, -1) },
      { id: `${id}:down`, label: `Move ${title(id)} later`, icon: 'arrow-down', run: () => moveCard(id, 1) },
    ]),
  }),
});

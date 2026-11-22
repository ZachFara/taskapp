import { defineCommand } from '../../registry';
import { emit } from '../_event';

defineCommand({
  id: 'cycle.move_unfinished',
  label: 'Move unfinished issues to the next cycle',
  icon: 'arrow-right',
  group: 'Actions',
  noShortcutReason: 'Rare and bulk; reached from the Cycles page and the command menu.',
  when: (ctx) => ctx.route.startsWith('/cycles'),
  run: emit('cycle.move_unfinished'),
});

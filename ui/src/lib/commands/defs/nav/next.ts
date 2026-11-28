import { defineCommand } from '../../registry';

defineCommand({
  id: 'nav.next',
  label: 'Go to Next actions',
  icon: 'flag',
  group: 'Navigation',
  shortcut: 'g n',
  run: (ctx) => ctx.navigate('/next'),
});

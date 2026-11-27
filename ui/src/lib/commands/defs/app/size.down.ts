import { defineCommand } from '../../registry';
import { stepInterfaceSize } from '../../../interfaceSize.svelte';

defineCommand({
  id: 'size.down',
  label: 'Decrease interface size',
  icon: 'minus',
  group: 'Settings',
  shortcut: 'mod+-',
  run: () => stepInterfaceSize(-1),
});

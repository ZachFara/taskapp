import { defineCommand } from '../../registry';
import { stepInterfaceSize } from '../../../interfaceSize.svelte';

defineCommand({
  id: 'size.up',
  label: 'Increase interface size',
  icon: 'plus',
  group: 'Settings',
  shortcut: 'mod+=',
  run: () => stepInterfaceSize(1),
});

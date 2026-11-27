import { defineCommand } from '../../registry';
import { DEFAULT, setInterfaceSize } from '../../../interfaceSize.svelte';

defineCommand({
  id: 'size.reset',
  label: 'Reset interface size',
  icon: 'undo',
  group: 'Settings',
  shortcut: 'mod+0',
  run: () => setInterfaceSize(DEFAULT),
});

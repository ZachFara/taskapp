import { defineCommand } from '../../registry';
import { runUndo } from '../../../../components/undoState.svelte';

// Not global: inside a text field mod+z stays the field's own undo.
defineCommand({
  id: 'history.undo',
  label: 'Undo',
  icon: 'undo',
  group: 'History',
  shortcut: ['mod+z', 'u'],
  run: (ctx) => runUndo(ctx.api),
});

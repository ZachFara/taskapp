import { defineCommand } from '../../registry';
import { refreshView } from '../issue/_issue';
import { runRedo } from '../../../../components/undoState.svelte';

// ctrl+r is not bound: the key grammar maps ctrl to mod, so it would also take cmd+r (reload).
defineCommand({
  id: 'history.redo',
  label: 'Redo',
  icon: 'undo',
  group: 'History',
  shortcut: 'mod+shift+z',
  run: (ctx) => runRedo(ctx.api).then(refreshView),
});

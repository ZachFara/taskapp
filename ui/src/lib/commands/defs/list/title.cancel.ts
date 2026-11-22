import { defineCommand } from '../../registry';
import { listState } from '../../../listState.svelte';

// Escape fires inside the text field too, which is how an inline rename is abandoned without a keydown listener.
defineCommand({
  id: 'list.title.cancel',
  label: 'Cancel rename',
  icon: 'close',
  group: 'List',
  shortcut: 'escape',
  scope: 'list-edit',
  when: () => listState.editing !== null,
  run: () => {
    listState.editing = null;
  },
});

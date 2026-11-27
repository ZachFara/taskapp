import { hasImageTarget, pickImage } from '../../images';
import { defineCommand } from '../registry';

// Offered while a description or comment box that takes images is on screen; it attaches to the one used last.
defineCommand({
  id: 'image.attach',
  label: 'Attach image',
  icon: 'plus',
  group: 'Actions',
  noShortcutReason: 'A button under every description and comment box, and the command menu.',
  when: () => hasImageTarget(),
  run: () => pickImage(),
});

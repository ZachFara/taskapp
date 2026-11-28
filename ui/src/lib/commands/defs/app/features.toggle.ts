import { FEATURES, features, setFeature } from '../../../features.svelte';
import { defineCommand } from '../../registry';

// One command per switch on Settings > Features.
for (const f of FEATURES) {
  defineCommand({
    id: `feature.${f.name}.toggle`,
    label: `Turn ${f.label} on or off`,
    icon: 'layers',
    group: 'Settings',
    noShortcutReason: 'Rare setting; the Features page and the command menu cover it.',
    run: () => setFeature(f.name, !features[f.name]),
  });
}

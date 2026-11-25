import { defineCommand } from '../../registry';
import { toggleShowProjects } from '../../../sidebarPrefs.svelte';

defineCommand({
  id: 'sidebar.showProjects', label: 'Show projects in the sidebar', icon: 'project', group: 'View',
  noShortcutReason: 'Rare setting, reached from the Projects header menu and the command menu.',
  run: () => toggleShowProjects(),
});

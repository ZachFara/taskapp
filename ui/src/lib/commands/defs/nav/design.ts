import { defineCommand } from '../../registry';

// Registered only in dev builds, so the design gallery is not reachable from a shipped app.
if (import.meta.env.DEV)
  defineCommand({
    id: 'nav.design',
    label: 'Go to Design gallery',
    icon: 'layers',
    group: 'Navigation',
    shortcut: 'g d',
    run: (ctx) => ctx.navigate('/design'),
  });

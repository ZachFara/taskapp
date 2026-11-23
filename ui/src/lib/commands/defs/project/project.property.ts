import { defineCommand } from '../../registry';
import { onProject } from '../_event';
import { setProject } from './_project';

// Select and checkbox properties only, like issue.property: values that need typing are edited on the page.
defineCommand({
  id: 'project.property', label: 'Set project property', icon: 'filter', group: 'Project',
  noShortcutReason: 'Occasional action; reached from the project page and the command menu.',
  when: onProject,
  page: (ctx) => ({
    title: 'Project property',
    items: ctx.api.properties.listDefs().then((defs) => defs.filter((d) => d.applies_to !== 'Issues').flatMap((d) => {
      const set = (v: string | boolean) => () => setProject(ctx, (p) => ({ properties: { ...p.properties, [d.key]: v } }));
      if (d.kind === 'Select') return d.options.map((o) => ({ id: `${d.key}:${o.value}`, label: `${d.name}: ${o.value}`, run: set(o.value) }));
      if (d.kind === 'Checkbox') return [{ id: `${d.key}:yes`, label: `${d.name}: yes`, run: set(true) }, { id: `${d.key}:no`, label: `${d.name}: no`, run: set(false) }];
      return [];
    })),
  }),
});

import type { FieldRef, ViewDef, ViewTemplate } from './bindings';

/** The display settings a stored view may lack; the backend fills them in the same way. */
export const withDisplay = (d: Partial<ViewDef> & Pick<ViewDef, 'layout'>): ViewDef => ({
  filter: null, sorts: [], group_by: null, columns: [], include_archived: false,
  density: 'comfortable', show_empty_groups: false, show_sub_issues: 'nest', card_fields: [], show_weekends: true,
  // JSON, not structuredClone: a def from a Svelte state proxy cannot be structured-cloned.
  ...JSON.parse(JSON.stringify(d)),
});

const fixed = (f: Extract<FieldRef, { Fixed: unknown }>['Fixed']): FieldRef => ({ Fixed: f });

/** The three starting templates (core's `view_templates::seed`): `keys` are the custom properties the planning table lists. */
export function seedViewTemplates(keys: string[], weekCutoff: string): ViewTemplate[] {
  const defs: [string, string, ViewDef][] = [
    ['Status board', 'board', withDisplay({
      layout: 'Board', group_by: fixed('Status'), card_fields: [fixed('Priority'), fixed('Label'), fixed('DueDate')],
    })],
    ['Planning table', 'list', withDisplay({
      layout: 'Table',
      columns: [fixed('Priority'), fixed('Status'), fixed('Project'), fixed('Estimate'), fixed('DueDate'), ...keys.map((k) => ({ Property: k }))],
    })],
    // shortcut: the 7 days are fixed when the data is made; saved filters have no relative dates yet.
    ['This week', 'calendar', withDisplay({
      layout: 'List', group_by: fixed('Project'), filter: { Cond: { field: fixed('DueDate'), op: 'Lte', value: weekCutoff } },
    })],
  ];
  return defs.map(([name, icon, def], i) => ({ id: i + 1, name, icon: `icon:${icon}`, color: null, def, position: i }));
}

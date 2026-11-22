import { defineCommand } from '../../registry';
import { choices, pageOf, patchOf } from './_issue';

type Row = [string, ReturnType<typeof patchOf>];
const set = (key: string, v: string | boolean): ReturnType<typeof patchOf> => patchOf({ properties: { [key]: v } });

// Select and checkbox properties only: values that need typing wait for a text input on pages.
defineCommand({
  id: 'issue.property', label: 'Set property', icon: 'filter', group: 'Issue', shortcut: 'a',
  page: (ctx) =>
    pageOf(ctx, 'Property', choices.defs.flatMap((d): Row[] =>
      d.kind === 'Select' ? d.options.map((o): Row => [`${d.name}: ${o.value}`, set(d.key, o.value)])
      : d.kind === 'Checkbox' ? [[`${d.name}: yes`, set(d.key, true)], [`${d.name}: no`, set(d.key, false)]]
      : [])),
});

// The undo step's label, built the way crates/commands/src/step_label.rs builds it. Arguments are positional.
type Obj = Record<string, unknown>;
const count = (n: number, noun: string) => `${n} ${noun}${n === 1 ? '' : 's'}`;
const len = (v: unknown) => (Array.isArray(v) ? v.length : 0);

function generic(name: string): string {
  const [plural, ...verb] = name.split('_');
  const module = plural.endsWith('ies') ? `${plural.slice(0, -3)}y` : plural.endsWith('ses') ? plural.slice(0, -2) : plural.replace(/s+$/, '');
  let text = verb.join(' ');
  if (!text.includes(module)) text = `${text} ${module}`;
  text = text.trim();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

function bulk(ids: unknown, patch: Obj): string {
  const n = len(ids);
  if (typeof patch.archive === 'boolean') return `${patch.archive ? 'Archived' : 'Restored'} ${count(n, 'issue')}`;
  const set = Object.entries((patch.patch ?? {}) as Obj).filter(([, v]) => v !== null && v !== undefined).map(([k]) => k);
  return set.length === 1 ? `Set ${set[0].replace(/_id$/, '')} on ${count(n, 'issue')}` : `Edited ${count(n, 'issue')}`;
}

/** `name` is the wire command (`labels_update`), `args` the call's arguments in order, `out` its result. */
export function stepLabel(name: string, args: unknown[], out: unknown): string {
  const key = String((out as Obj | null)?.key ?? 'issue');
  switch (name) {
    case 'issues_create': case 'issues_create_from_quickadd': return `Created ${key}`;
    case 'issues_update': return `Edited ${key}`;
    case 'issues_bulk_update': return bulk(args[0], (args[1] ?? {}) as Obj);
    case 'issues_duplicate': return `Duplicated ${count(len(args[0]), 'issue')}`;
    case 'import_apply': return `Imported ${count(Number((out as Obj).created ?? 0), 'issue')}`;
    case 'pins_set': return `${args[2] === false ? 'Unpinned' : 'Pinned'} ${count(len(args[1]), 'item')}`;
    case 'project_statuses_create': return 'Created project status';
    case 'project_statuses_update': return 'Edited project status';
    case 'project_statuses_reorder': return 'Reordered project statuses';
    case 'project_statuses_delete': return 'Deleted project status';
    case 'folders_place': return 'Moved folder';
    case 'folders_place_item': return `Moved ${typeof args[0] === 'string' ? args[0] : 'item'}`;
    case 'relation_kinds_create': return 'Created relation kind';
    case 'relation_kinds_update': return 'Edited relation kind';
    case 'relation_kinds_delete': return 'Deleted relation kind';
    case 'views_save':
    case 'views_save_project': return args[0] === null || args[0] === undefined ? `Saved view ${args[1]}` : `Changed display of ${args[1]}`;
    default: return generic(name);
  }
}

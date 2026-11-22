// One table, run in order against the mock and a real devserver on the same seed. Cases build on earlier ones.
export type Case = {
  name: string;
  cmd: string;
  args?: Record<string, unknown>;
  /** TODO(P3): the backend command is not on this branch yet. The mock must return `expectMock`; the real side must
   * still answer NotFound. When P3 lands, delete `pending` and `expectMock` and the case compares both sides. */
  pending?: { by: string; expectMock: unknown };
};

const cond = (field: unknown, op: string, value: unknown = null) => ({ Cond: { field, op, value } });
const fixed = (f: string) => ({ Fixed: f });
const prop = (p: string) => ({ Property: p });
const def = (extra: Record<string, unknown> = {}) => ({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false, ...extra });
const query = (name: string, d: Record<string, unknown>): Case => ({ name, cmd: 'views_query', args: { def: def(d) } });
const filter = (name: string, f: unknown, extra: Record<string, unknown> = {}) => query(name, { filter: f, ...extra });
const call = (name: string, cmd: string, args: Record<string, unknown> = {}): Case => ({ name, cmd, args });
const tpl = (name: string, slug: string, body: Record<string, unknown>) => ({ name, slug, icon: null, color: null, body });

export const cases: Case[] = [
  // Fixtures and workspace
  call('fixtures: issue key uses the workspace prefix', 'issues_get_by_key', { key: 'ABC-1' }),
  call('fixtures: projects', 'projects_list'),
  call('fixtures: statuses carry colour key and hex', 'statuses_list'),
  call('fixtures: seeded views', 'views_list'),
  call('fixtures: labels', 'labels_list'),
  call('fixtures: cycles', 'cycles_list'),
  call('fixtures: property defs', 'properties_list_defs'),

  // Saved-view filters
  filter('filter: status Eq', cond(fixed('Status'), 'Eq', 2)),
  filter('filter: status In', cond(fixed('Status'), 'In', [1, 3])),
  filter('filter: status NotIn', cond(fixed('Status'), 'NotIn', [1, 2, 3])),
  filter('filter: status category In (migration form)', cond(fixed('StatusCategory'), 'In', ['unstarted', 'started'])),
  filter('filter: status category Neq', cond(fixed('StatusCategory'), 'Neq', 'completed')),
  filter('filter: project Eq', cond(fixed('Project'), 'Eq', 3)),
  filter('filter: project IsEmpty', cond(fixed('Project'), 'IsEmpty')),
  filter('filter: priority Gte with sort', cond(fixed('Priority'), 'Gte', 3), { sorts: [{ field: fixed('Priority'), descending: true }] }),
  filter('filter: start date Gt', cond(fixed('StartDate'), 'Gt', '2026-12-05')),
  filter('filter: due date Lte', cond(fixed('DueDate'), 'Lte', '2026-12-10')),
  filter('filter: title Contains is case-insensitive', cond(fixed('Title'), 'Contains', 'TASK 1')),
  filter('filter: custom select property', cond(prop('area'), 'Eq', 'work')),
  filter('filter: custom number property', cond(prop('effort'), 'Gt', 10)),
  filter('filter: custom property IsNotEmpty', cond(prop('effort'), 'IsNotEmpty')),
  filter('filter: unknown property is Invalid', cond(prop('nope'), 'Eq', 1)),
  filter('filter: And Or Not', { And: [{ Or: [cond(fixed('Priority'), 'Eq', 1), cond(fixed('Priority'), 'Eq', 2)] }, { Not: cond(fixed('Project'), 'Eq', 1) }] }),
  call('labels: add to issue 1', 'labels_add_label', { issue: 1, label: 1 }),
  call('labels: add to issue 4', 'labels_add_label', { issue: 4, label: 1 }),
  call('labels: add second label to issue 4', 'labels_add_label', { issue: 4, label: 2 }),
  filter('filter: label Eq', cond(fixed('Label'), 'Eq', 1)),
  filter('filter: label In', cond(fixed('Label'), 'In', [2, 3])),
  filter('filter: label IsEmpty', cond(fixed('Label'), 'IsEmpty')),
  call('archived: archive issue 3', 'issues_archive', { id: 3 }),
  query('archived: hidden by default', {}),
  query('archived: included on request', { include_archived: true }),
  filter('archived: Archived Eq true', cond(fixed('Archived'), 'Eq', true), { include_archived: true }),

  // Sorting and grouping
  query('sort: title descending', { sorts: [{ field: fixed('Title'), descending: true }] }),
  query('sort: two keys', { sorts: [{ field: fixed('Priority'), descending: false }, { field: fixed('Status'), descending: true }] }),
  query('sort: nulls last', { sorts: [{ field: fixed('Project'), descending: false }] }),
  query('sort: custom property', { sorts: [{ field: prop('effort'), descending: true }] }),
  call('group: save a grouped board view', 'views_save', { id: null, name: 'By project', def: def({ group_by: fixed('Project'), layout: 'Board' }), icon: 'icon:folder', color: 'teal' }),
  call('group: saved view lists its definition', 'views_list'),
  call('group: reorder views', 'views_reorder', { ids: [4, 1, 2, 3] }),
  call('group: bad view colour is Invalid', 'views_save', { id: null, name: 'Bad', def: def(), icon: null, color: 'teal2' }),

  // Quick add
  call('quickadd: relative date', 'issues_parse_quickadd', { input: 'buy milk tomorrow' }),
  call('quickadd: priority project label estimate', 'issues_parse_quickadd', { input: 'call dentist p1 #website @Bug ~2h by fri' }),
  call('quickadd: time of day', 'issues_parse_quickadd', { input: 'standup at 9:15am' }),
  call('quickadd: month and day, start', 'issues_parse_quickadd', { input: 'plan trip start dec 3 due 25 dec' }),
  call('quickadd: in N weeks', 'issues_parse_quickadd', { input: 'review in 2 weeks' }),
  call('quickadd: tokens only keeps the text', 'issues_parse_quickadd', { input: 'p2 ~1d' }),
  call('quickadd: unknown project and label stay in the title', 'issues_create_from_quickadd', { input: 'fix thing #nope @Nolabel p3' }),
  call('quickadd: known project and label', 'issues_create_from_quickadd', { input: 'write copy #mobile @design p2 ~30m' }),
  call('quickadd: unknown template is Invalid', 'issues_create_from_quickadd', { input: '/nosuch hello' }),
  call('templates: create', 'templates_create', {
    input: tpl('Bug report', 'bug', { title_prefix: 'Bug: ', fields: { priority: 2, label_ids: [1], project_id: 1, status_id: 2, estimate_minutes: null, description: 'Steps:', properties: {} }, sub_issues: [{ title: 'Reproduce', fields: { priority: 1 } }] }),
  }),
  call('quickadd: /template applies the template', 'issues_create_from_quickadd', { input: '/bug login fails #mobile' }),
  call('templates: list', 'templates_list'),
  call('templates: apply with overrides', 'templates_apply', { id: 1, overrides: { title: 'Crash', priority: 4 } }),
  call('templates: update', 'templates_update', { id: 1, input: tpl('Defect', 'defect', { title_prefix: '', fields: {}, sub_issues: [] }) }),
  call('templates: delete', 'templates_delete', { id: 1 }),

  // Search
  call('search: title words', 'search_search', { text: 'fixture task 12', limit: 10 }),
  call('search: limit', 'search_search', { text: 'fixture', limit: 3 }),
  call('search: no match', 'search_search', { text: 'zzzz', limit: 5 }),

  // Pins
  call('pins: pin an issue', 'pins_pin', { kind: 'Issue', target: 'ABC-1', title: null }),
  call('pins: pin a project with a title', 'pins_pin', { kind: 'Project', target: 'website', title: 'Site' }),
  call('pins: pin a view', 'pins_pin', { kind: 'View', target: '1', title: null }),
  call('pins: pinning twice returns the same pin', 'pins_pin', { kind: 'Issue', target: 'ABC-1', title: null }),
  call('pins: reorder', 'pins_reorder', { ids_in_order: [3, 1, 2] }),
  call('pins: resolved list', 'pins_list_resolved'),
  call('pins: unpin', 'pins_unpin', { id: 2 }),

  // Cycles
  call('cycles: create unnamed', 'cycles_create', { name: null, start_date: '2026-12-15', end_date: '2026-12-28', icon: 'icon:repeat', color: 'blue' }),
  call('cycles: end before start is Invalid', 'cycles_create', { name: 'Bad', start_date: '2026-12-20', end_date: '2026-12-10', icon: null, color: null }),
  call('cycles: list', 'cycles_list'),
  call('cycles: assign an issue', 'issues_update', { id: 2, patch: { cycle_id: 2 } }),
  filter('cycles: filter by cycle', cond(fixed('Cycle'), 'Eq', 2)),
  call('cycles: delete clears the issue', 'cycles_delete', { id: 2 }),
  call('cycles: issue lost its cycle', 'issues_get', { id: 2 }),

  // Relations and schedule
  call('relations: add Blocks', 'relations_add', { rel: { from: 4, to: 8, kind: 'Blocks' } }),
  call('relations: add Related', 'relations_add', { rel: { from: 4, to: 12, kind: 'Related' } }),
  call('relations: adding twice is a no-op', 'relations_add', { rel: { from: 4, to: 8, kind: 'Blocks' } }),
  call('relations: unknown issue is NotFound', 'relations_add', { rel: { from: 4, to: 999, kind: 'Blocks' } }),
  call('relations: for issue', 'relations_for_issue', { id: 4 }),
  call('schedule: bars for the active view', 'schedule_bars', { view_id: 1 }),
  call('schedule: critical path', 'schedule_critical_path', { view_id: 3 }),
  call('schedule: shift dependents', 'schedule_shift_dependents', { moved: { issue_id: 4, start_date: '2026-12-04', due_date: '2026-12-09' } }),
  call('relations: remove', 'relations_remove', { rel: { from: 4, to: 12, kind: 'Related' } }),

  // Undo and redo
  call('undo: create an issue', 'issues_create', { input: { title: 'Undo me' } }),
  call('undo: status names the step', 'history_undo_status'),
  call('undo: undo', 'history_undo'),
  call('undo: the issue is gone', 'issues_get_by_key', { key: 'ABC-21' }),
  call('undo: redo', 'history_redo'),
  call('undo: the issue is back', 'issues_get_by_key', { key: 'ABC-21' }),
  call('undo: nothing left to redo', 'history_redo'),
  call('undo: update an issue', 'issues_update', { id: 5, patch: { title: 'Renamed', priority: 4 } }),
  call('undo: undo the update', 'history_undo'),
  call('undo: title is back', 'issues_get', { id: 5 }),

  // Icons and colours
  call('colour: hex maps to the nearest key', 'labels_create', { name: 'Hot', color: '#ff0000', icon: null }),
  call('colour: palette key with emoji icon', 'labels_create', { name: 'Fun', color: 'pink', icon: 'emoji:🚀' }),
  call('colour: unknown key is Invalid', 'labels_create', { name: 'Odd', color: 'teal2', icon: null }),
  call('icon: bad shape is Invalid', 'projects_create', { slug: 'x', name: 'X', icon: 'rocket', color: null }),
  call('colour: project with icon and colour', 'projects_create', { slug: 'garden', name: 'Garden', icon: 'icon:leaf', color: 'green' }),
  call('colour: status with colour hex', 'statuses_create', { name: 'Review', category: 'Started', color: '#f2c94c', icon: null }),

  // Import
  call('import: csv plan maps the title column', 'import_csv_plan', { csv: 'Title,Notes\nOrder desks,urgent\nBook movers,\n', mapping: null }),
  call('import: csv plan with explicit mapping', 'import_csv_plan', { csv: 'Name,Summary\nA,Paint walls\n', mapping: [['Summary', 'title']] }),
  call('import: csv without a title column is Invalid', 'import_csv_plan', { csv: 'Foo,Bar\n1,2\n', mapping: null }),
  call('import: apply a plan', 'import_apply', { plan: { source: 'csv', issues: [{ external_id: 'csv:1', title: 'Imported one', description: '', status: null, priority: 0, project: null, labels: [], start_date: null, due_date: null, estimate_minutes: null, properties: {}, parent: null, cycle: null }] } }),
  filter('import: imported issue is findable', cond(fixed('Title'), 'Contains', 'imported')),

  // Writes and their validation
  call('issues: empty title is Invalid', 'issues_create', { input: { title: '  ' } }),
  call('issues: priority above 4 is Invalid', 'issues_update', { id: 6, patch: { priority: 9 } }),
  call('issues: unknown status is Invalid', 'issues_update', { id: 6, patch: { status_id: 99 } }),
  call('issues: a status change to Done sets completed_at', 'issues_update', { id: 6, patch: { status_id: 4 } }),
  call('issues: moving back clears completed_at', 'issues_update', { id: 6, patch: { status_id: 3 } }),
  call('issues: parent cycle is Invalid', 'issues_update', { id: 7, patch: { parent_id: 7 } }),
  call('issues: set a parent', 'issues_update', { id: 8, patch: { parent_id: 7 } }),
  call('issues: children', 'issues_children', { id: 7 }),
  call('issues: timed due date', 'issues_update', { id: 9, patch: { due: { DateTime: { utc: '2026-12-31T23:30:00Z', tz: 'Asia/Tokyo' } } } }),
  filter('filter: timed due counts as its local day', cond(fixed('DueDate'), 'Eq', '2027-01-01')),
  call('issues: an empty patch changes nothing', 'issues_update', { id: 6, patch: {} }),
  call('issues: get unknown key is NotFound', 'issues_get_by_key', { key: 'ABC-999' }),
  call('issues: restore', 'issues_restore', { id: 3 }),
  call('properties: wrong kind of value is Invalid', 'properties_set_value', { issue: 1, key: 'effort', value: 'lots' }),
  call('properties: set a value', 'properties_set_value', { issue: 1, key: 'effort', value: 7 }),
  call('properties: select option must exist', 'properties_set_value', { issue: 1, key: 'area', value: 'moon' }),
  call('properties: create a def', 'properties_create_def', { def: { key: 'link', name: 'Link', kind: 'Url', options: [], project_id: null, position: 3 } }),
  call('properties: duplicate key is Conflict', 'properties_create_def', { def: { key: 'link', name: 'Link', kind: 'Url', options: [], project_id: null, position: 3 } }),
  call('properties: list defs', 'properties_list_defs'),
  call('comments: add', 'comments_add_comment', { issue: 1, body: 'First note' }),
  call('comments: list', 'comments_list', { issue: 1 }),
  call('comments: searchable', 'search_search', { text: 'first note', limit: 5 }),
  call('milestones: create', 'projects_create_milestone', { project: 1, name: 'Beta', target_date: '2027-01-15' }),
  call('milestones: list', 'projects_list_milestones', { project: 1 }),
  call('projects: progress', 'projects_progress', { id: 1 }),
  call('projects: duplicate slug is Conflict', 'projects_create', { slug: 'mobile', name: 'Again', icon: null, color: null }),
  call('labels: duplicate name is Conflict', 'labels_create', { name: 'Bug', color: 'red', icon: null }),
  call('statuses: delete moves issues', 'statuses_delete', { id: 5, move_issues_to: 1 }),
  call('statuses: unknown target is NotFound', 'statuses_delete', { id: 4, move_issues_to: 99 }),
  call('history: changes of an issue', 'history_list_for', { entity: 'issue', entity_id: '5' }),
  call('relations: self relation is Invalid', 'relations_add', { rel: { from: 4, to: 4, kind: 'Related' } }),
  call('relations: blocking cycle is Conflict', 'relations_add', { rel: { from: 8, to: 4, kind: 'Blocks' } }),
  call('quickadd: weekday and time', 'issues_parse_quickadd', { input: 'sync with team next week at 15:30 p0' }),
  call('quickadd: slash text is plain title', 'issues_parse_quickadd', { input: '/usr/bin broke' }),
  call('settings: round trip', 'settings_set', { key: 'theme', value: 'dark' }),
  call('settings: read back', 'settings_get', { key: 'theme' }),
  call('settings: unknown key is null', 'settings_get', { key: 'nothing' }),

  // Workspace: last, because setup changes the issue key for everything after it
  { name: 'workspace_status of a set-up workspace', cmd: 'workspace_status', pending: { by: 'P3', expectMock: { setup_done: true, name: 'Cairn', key: 'ABC', timezone: 'UTC' } } },
  {
    name: 'workspace_setup renames and rekeys', cmd: 'workspace_setup', args: { name: 'Studio', key: 'STU', timezone: 'Europe/Berlin' },
    pending: { by: 'P3', expectMock: { setup_done: true, name: 'Studio', key: 'STU', timezone: 'Europe/Berlin' } },
  },
];

<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Label, Placement, Project, ProjectStatus, PropertyDef, ViewFolder } from '../lib/bindings';
  import { buildRows } from '../lib/folderTree';
  import Icon from '../ui/Icon.svelte';
  import { arrange, glyphCategory, noFilters, viewDefOf, type Group, type Sort } from '../lib/projectFields';
  import { PRIORITIES, priorityGlyph } from '../lib/priority';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import RowActions from '../ui/RowActions.svelte';
  import IconColorPicker from './pickers/IconColorPicker.svelte';
  import { rowActs, rowUi } from '../lib/rowUi.svelte';
  import { patch } from '../lib/rowActions';
  import { navigate } from '../app/routes';
  import { refreshAll } from '../lib/stores.svelte';
  import { dayText } from './views/dates';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import PriorityGlyph from '../ui/PriorityGlyph.svelte';
  import Select from '../ui/Select.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import ProjectStatusPicker from './project/ProjectStatusPicker.svelte';

  type Row = Project & { done: number; total: number };
  let rows = $state<Row[]>([]);
  let loaded = $state(false);
  // Slugs of pinned projects.
  let pinned = $state<Set<string>>(new Set());
  let creating = $state(false);
  let name = $state('');
  let statuses = $state<ProjectStatus[]>([]);
  let labels = $state<Label[]>([]);
  let folders = $state<ViewFolder[]>([]);
  let places = $state<Placement[]>([]);
  const filtered = $derived(group !== 'none' || fStatus !== '' || fPriority !== '' || fLabel !== '');
  let defs = $state<PropertyDef[]>([]);
  // The toolbar: what to show, in what order, grouped how. Select values are strings, '' meaning no filter.
  let sort = $state<Sort>('name');
  let group = $state<Group>('none');
  let fStatus = $state('');
  let fPriority = $state('');
  let fLabel = $state('');
  const num = (v: string) => (v === '' ? null : Number(v));
  const groups = $derived(arrange(rows, statuses, { sort, group, filters: { ...noFilters, status: num(fStatus), priority: num(fPriority), label: num(fLabel) } }));
  const propCols = $derived(defs.filter((d) => d.applies_to !== 'Issues'));
  const shown = (v: unknown) => (v === undefined || v === null || v === '' ? '' : Array.isArray(v) ? v.join(', ') : v === true ? 'Yes' : v === false ? 'No' : String(v));
  const any = (label: string) => ({ value: '', label });

  // A project view with the page's current filters, sort and grouping; it opens as a table.
  async function saveAsView() {
    const v = await api.views.saveProject(null, filtered ? 'Projects view' : 'All projects', viewDefOf(sort, group, { ...noFilters, status: num(fStatus), priority: num(fPriority), label: num(fLabel) }), null, null);
    await refreshAll();
    navigate(`/view/${v.id}`);
  }

  $effect(() => {
    const open = () => (creating = true);
    addEventListener('command:project.new', open);
    addEventListener('command:projects.saveAsView', saveAsView);
    return () => (removeEventListener('command:project.new', open), removeEventListener('command:projects.saveAsView', saveAsView));
  });

  async function load() {
    const list = await api.projects.list();
    const progress = await Promise.all(list.map((p) => api.projects.progress(p.id)));
    rows = list.map((p, i) => ({ ...p, done: progress[i][0], total: progress[i][1] }));
    [statuses, labels, defs, folders, places] = await Promise.all([api.project.statusesList(), api.labels.list(), api.properties.listDefs(), api.folders.list(), api.folders.items()]);
    pinned = new Set((await api.pins.listResolved()).filter((r) => r.pin.kind === 'Project').map((r) => r.pin.target));
    loaded = true;
  }

  const slugOf = (s: string) => s.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

  async function create() {
    const n = name.trim();
    if (!n) return;
    await api.projects.create(slugOf(n), n, null, null);
    name = '';
    creating = false;
    await load();
  }

  onMount(() => {
    void load();
    addEventListener('rows:changed', load);
    return () => removeEventListener('rows:changed', load);
  });
</script>

<PageHeader title="Projects" icon="icon:project">
  {#snippet actions()}
    <a class="timeline" href="/projects/timeline">All projects timeline</a>
    <Button icon="layers" onclick={saveAsView}>Save as view</Button>
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New project</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if !loaded}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0}
    <EmptyState title="No projects yet" hint="A project groups issues toward a target date.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={() => (creating = true)}>New project</Button>{/snippet}
    </EmptyState>
  {:else}
    <div class="bar" role="group" aria-label="Arrange projects">
      <Select label="Group by" bind:value={group as string} items={[{ value: 'none', label: 'No grouping' }, { value: 'status', label: 'Group by status' }, { value: 'priority', label: 'Group by priority' }]} />
      <Select label="Sort by" bind:value={sort as string} items={[{ value: 'name', label: 'Sort by name' }, { value: 'status', label: 'Sort by status' }, { value: 'priority', label: 'Sort by priority' }, { value: 'target', label: 'Sort by target date' }]} />
      <Select label="Filter by status" bind:value={fStatus} items={[any('Any status'), ...statuses.map((s) => ({ value: String(s.id), label: s.name, glyph: { icon: s.icon, color: s.color } }))]} />
      <Select label="Filter by priority" bind:value={fPriority} items={[any('Any priority'), ...PRIORITIES.map((label, i) => ({ value: String(i), label }))]} />
      <Select label="Filter by label" bind:value={fLabel} items={[any('Any label'), ...labels.map((l) => ({ value: String(l.id), label: l.name, glyph: { icon: l.icon, color: l.color } }))]} />
    </div>
    <table class="table">
      <thead>
        <tr>
          <th scope="col">Project</th><th scope="col">Status</th><th scope="col">Priority</th><th scope="col">Labels</th><th scope="col">Progress</th><th scope="col">Target</th>
          {#each propCols as d (d.key)}<th scope="col">{d.name}</th>{/each}
          <th scope="col" class="num">Issues</th>
        </tr>
      </thead>
      {#each groups as g (g.key)}
        <tbody>
          {#if group !== 'none'}
            <tr class="group"><th scope="rowgroup" colspan={7 + propCols.length}>
              <span class="cell">{#if g.status}<StatusGlyph category={glyphCategory[g.status.category]} />{/if}{g.name}<span class="muted">{g.rows.length}</span></span>
            </th></tr>
          {/if}
          <!-- Folders (nested) around the projects; the page's own sort orders the projects inside each. -->
          {#each buildRows('project', folders, places, g.rows, { hideEmpty: filtered }) as r (r.type === 'folder' ? `f${r.folder.id}` : `p${r.item.id}`)}
            {#if r.type === 'folder'}
              <tr class="fold" data-folder-id={r.folder.id}><td colspan={7 + propCols.length} style:padding-left={`calc(var(--s-4) + ${r.depth} * var(--s-8))`}><span class="cell"><Icon name="folder" size={12} />{r.folder.name}<span class="muted">{r.count}</span></span></td></tr>
            {:else}
            {@const p = r.item}
            <tr data-slug={p.slug} onclick={() => navigate(`/project/${p.slug}`)}>
              <td style:padding-left={`calc(var(--s-4) + ${r.depth} * var(--s-8))`}>
                <div role="presentation" onclick={(e) => e.stopPropagation()}>
                  <RowActions label={p.name} actions={rowActs({ kind: 'project', id: p.id, name: p.name }, pinned.has(p.slug))}>
                    <IconColorPicker label={`Icon and colour of ${p.name}`} value={{ icon: p.icon ?? 'icon:project', color: p.color }} onchange={(c) => patch({ kind: 'project', id: p.id, name: p.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `project:${p.id}`, (o) => (rowUi.picking = o ? `project:${p.id}` : null)} />
                    <a class="name" href={`/project/${p.slug}`} onclick={(e) => (e.preventDefault(), navigate(`/project/${p.slug}`))}>{p.name}</a>
                  </RowActions>
                </div>
              </td>
              <td><ProjectStatusPicker project={p} {statuses} onsaved={load} /></td>
              <td><span class="cell"><PriorityGlyph level={priorityGlyph(p.priority)} />{p.priority ? PRIORITIES[p.priority] : ''}</span></td>
              <td><span class="cell">{#each labels.filter((l) => p.label_ids.includes(l.id)) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{/each}</span></td>
              <td>
                <span class="prog"><span class="meter" aria-label={`${p.done} of ${p.total} done`}><span class="fill" style:width={`${p.total ? (p.done / p.total) * 100 : 0}%`}></span></span><span class="muted">{p.done} of {p.total}</span></span>
              </td>
              <td class="muted">{p.target_date ? dayText(p.target_date) : 'No target'}</td>
              {#each propCols as d (d.key)}<td class="muted">{shown(p.properties[d.key])}</td>{/each}
              <td class="num">{p.total}</td>
            </tr>
            {/if}
          {/each}
        </tbody>
      {/each}
    </table>
    {#if groups.length === 0}<p class="muted none">No project matches these filters.</p>{/if}
  {/if}
</main>

<Dialog bind:open={creating} title="New project">
  {#snippet children()}
    <TextInput bind:value={name} label="Project name" placeholder="Project name" />
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (creating = false)}>Cancel</Button>
    <Button variant="primary" onclick={create}>Create project</Button>
  {/snippet}
</Dialog>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); }
  .timeline {
    display: inline-flex; align-items: center; height: var(--ctl-h); padding: 0 var(--s-5);
    border: 1px solid var(--control); border-radius: var(--r-ctl); background: var(--sheet);
    color: var(--ink); font-weight: 500; text-decoration: none;
  }
  .timeline:hover { border-color: var(--muted); }
  .table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--fs-xs); font-weight: 600; color: var(--muted); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); }
  td { height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  tbody tr:hover { background: var(--active); }
  .name { display: inline-flex; align-items: center; gap: var(--s-4); font-weight: 600; color: inherit; text-decoration: none; }
  .prog { display: inline-flex; align-items: center; gap: var(--s-4); }
  tbody tr { cursor: pointer; }
  .muted { color: var(--muted); }
  .bar { display: flex; flex-wrap: wrap; gap: var(--s-4); padding: var(--s-5) 0; }
  .cell { display: inline-flex; align-items: center; gap: var(--s-3); flex-wrap: wrap; }
  .group th { padding: var(--s-5) var(--s-4) var(--s-3); background: var(--band-1); color: var(--ink); font-size: var(--fs-md); }
  .group .muted { margin-left: var(--s-4); font-weight: 500; }
  .fold td { color: var(--muted); font-weight: 600; background: var(--band-1); }
  .none { padding: var(--s-6) var(--s-4); }
  .num { text-align: right; font-variant-numeric: tabular-nums; }
  .meter { display: block; width: 160px; height: 6px; border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
</style>

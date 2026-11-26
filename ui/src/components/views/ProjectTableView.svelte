<script lang="ts">
  // The Table layout of a project view: the columns come from Display, a click on a row opens the project.
  import { navigate } from '../../app/routes';
  import type { FieldRef, SavedView, Sort } from '../../lib/bindings';
  import { nextSort, sameField, sortable } from '../../lib/viewSort';
  import SortHeading from './SortHeading.svelte';
  import { priorityGlyph } from '../../lib/priority';
  import { fieldName, groupRows, shownFields, statusGlyph } from '../../lib/projectViews';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import ProjectField from './ProjectField.svelte';
  import { projectData } from './projectData.svelte';

  let { view, onsort }: { view: SavedView; onsort?: (field: FieldRef) => void } = $props();
  let localSort = $state.raw<Sort[] | null>(null);
  const currentDef = $derived({ ...view.def, sorts: localSort ?? view.def.sorts });
  const activeSort = $derived(currentDef.sorts[0] ?? null);
  const sortHeading = (field: FieldRef) => onsort ? onsort(field) : (localSort = nextSort(localSort, field));
  const sortAria = (field: FieldRef) => activeSort && sameField(activeSort.field, field) ? (activeSort.descending ? 'descending' : 'ascending') : undefined;
  const data = projectData(() => currentDef);
  const fields = $derived(shownFields(view.def));
  const groups = $derived(groupRows(data.rows, view.def.group_by, data.ctx, { empty: view.def.show_empty_groups }));
</script>

{#if !data.loaded}
  <div class="loading" aria-busy="true">{#each [0, 1, 2, 3] as k (k)}<Skeleton height="var(--s-7)" width={`${90 - k * 6}%`} />{/each}</div>
{:else if data.rows.length === 0}
  <div class="empty"><EmptyState title="No projects in this view" hint="Projects that match this view will show up here." /></div>
{:else}
  <table class="table" aria-label={view.name} data-density={view.def.density === 'compact' ? 'compact' : undefined}>
    <thead>
      <tr>
        <th scope="col" aria-sort={sortAria({ Fixed: 'Title' })}><SortHeading field={{ Fixed: 'Title' }} label="Project" sort={activeSort} onclick={sortHeading} /></th>
        {#each fields as f (f.Property ?? f.Fixed)}<th scope="col" aria-sort={sortAria(f)}>{#if sortable(f, 'project')}<SortHeading field={f} label={fieldName(f, data.ctx.defs)} sort={activeSort} onclick={sortHeading} />{:else}{fieldName(f, data.ctx.defs)}{/if}</th>{/each}
      </tr>
    </thead>
    {#each groups as g (g.key)}
      <tbody>
        {#if g.name}
          <tr class="group"><th scope="rowgroup" colspan={1 + fields.length}>
            <span class="cell">{#if g.status}<StatusGlyph category={statusGlyph(g.status)} />{/if}{#if g.priority !== null}<PriorityGlyph level={priorityGlyph(g.priority)} />{/if}{g.name}<span class="muted">{g.rows.length}</span></span>
          </th></tr>
        {/if}
        {#each g.rows as r (r.project.id)}
          {@const p = r.project}
          <tr data-slug={p.slug} onclick={() => navigate(`/project/${p.slug}`)}>
            <td><span class="cell name"><EntityIcon icon={p.icon ?? 'icon:project'} color={p.color} size={14} /><a href={`/project/${p.slug}`} onclick={(e) => (e.preventDefault(), e.stopPropagation(), navigate(`/project/${p.slug}`))}>{p.name}</a></span></td>
            {#each fields as f (f.Property ?? f.Fixed)}<td><ProjectField field={f} row={r} ctx={data.ctx} /></td>{/each}
          </tr>
        {/each}
      </tbody>
    {/each}
  </table>
{/if}

<style>
  .loading { display: grid; gap: var(--s-5); padding: var(--s-6) var(--s-7); }
  .empty { padding-top: var(--s-10); }
  .table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--fs-xs); font-weight: 600; color: var(--muted); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); }
  td { height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  [data-density='compact'] td { height: var(--row-h-compact); }
  tbody tr { cursor: pointer; }
  tbody tr:hover { background: var(--active); }
  .name { font-weight: 600; }
  .name a { color: inherit; text-decoration: none; }
  .cell { display: inline-flex; align-items: center; gap: var(--s-3); flex-wrap: wrap; }
  .muted { color: var(--muted); margin-left: var(--s-4); font-weight: 500; }
  .group th { padding: var(--s-5) var(--s-4) var(--s-3); background: var(--band-1); color: var(--ink); font-size: var(--fs-md); }
</style>

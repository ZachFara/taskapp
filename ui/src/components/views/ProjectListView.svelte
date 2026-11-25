<script lang="ts">
  // The List layout of a project view: one row per project in bands, a click opens the project.
  import { navigate } from '../../app/routes';
  import type { SavedView } from '../../lib/bindings';
  import { priorityGlyph } from '../../lib/priority';
  import { fieldName, groupRows, shownFields, statusGlyph } from '../../lib/projectViews';
  import EmptyState from '../../ui/EmptyState.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import ProjectField from './ProjectField.svelte';
  import { projectData } from './projectData.svelte';

  let { view }: { view: SavedView } = $props();
  const data = projectData(() => view.def);
  const fields = $derived(shownFields(view.def));
  const groups = $derived(groupRows(data.rows, view.def.group_by, data.ctx, { empty: view.def.show_empty_groups }));
  let closed = $state<string[]>([]);
  const toggle = (k: string) => (closed = closed.includes(k) ? closed.filter((x) => x !== k) : [...closed, k]);
  const compact = $derived(view.def.density === 'compact');
</script>

{#if !data.loaded}
  <div class="loading" aria-busy="true">{#each [0, 1, 2, 3] as k (k)}<Skeleton height="var(--s-7)" width={`${90 - k * 6}%`} />{/each}</div>
{:else if data.rows.length === 0}
  <div class="empty"><EmptyState title="No projects in this view" hint="Projects that match this view will show up here." /></div>
{:else}
  <div class="wrap" style:--cols={`repeat(${fields.length}, minmax(0, 1fr))`}>
  <div class="heads" aria-hidden="true">
    <span></span><span></span><span>Project</span>
    {#each fields as f (f.Property ?? f.Fixed)}<span>{fieldName(f, data.ctx.defs)}</span>{/each}
  </div>
  <ul class="list" aria-label={view.name}>
    {#each groups as g (g.key)}
      {#if g.name}
        <li class="head">
          <GroupHeader name={g.name} count={g.rows.length} tone={g.priority ? (Math.min(4, g.priority) as 1 | 2 | 3 | 4) : 1} category={g.status ? statusGlyph(g.status) : undefined} icon={g.status?.icon} color={g.status?.color} collapsed={closed.includes(g.key)} ontoggle={() => toggle(g.key)}>
            {#snippet leading()}{#if g.priority !== null}<PriorityGlyph level={priorityGlyph(g.priority)} />{/if}{/snippet}
          </GroupHeader>
        </li>
      {/if}
      {#if !closed.includes(g.key)}
        {#each g.rows as r (r.project.id)}
          {@const p = r.project}
          {@const st = data.ctx.statuses.find((s) => s.id === p.status_id)}
          <li class="row" class:compact data-slug={p.slug} onclick={() => navigate(`/project/${p.slug}`)} role="row">
            <span class="st">{#if st}<StatusGlyph category={statusGlyph(st)} icon={st.icon} color={st.color} label={st.name} />{/if}</span>
            <span class="prio"><PriorityGlyph level={priorityGlyph(p.priority)} /></span>
            <span class="name"><EntityIcon icon={p.icon ?? 'icon:project'} color={p.color} size={14} /><a href={`/project/${p.slug}`} onclick={(e) => (e.preventDefault(), e.stopPropagation(), navigate(`/project/${p.slug}`))}>{p.name}</a></span>
            {#each fields as f (f.Property ?? f.Fixed)}<span class="cell"><ProjectField field={f} row={r} ctx={data.ctx} /></span>{/each}
          </li>
        {/each}
      {/if}
    {/each}
  </ul>
  </div>
{/if}

<style>
  .loading { display: grid; gap: var(--s-5); padding: var(--s-6) var(--s-7); }
  .empty { padding-top: var(--s-10); }
  .heads, .row { display: grid; grid-template-columns: var(--s-7) var(--s-7) minmax(0, 2fr) var(--cols); align-items: center; column-gap: var(--s-4); padding: 0 var(--s-7) 0 var(--s-4); }
  .heads { height: var(--row-h-compact); border-bottom: 1px solid var(--border); color: var(--muted); font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }
  .list { list-style: none; margin: 0; padding: 0; }
  .row { height: var(--row-h); border-bottom: 1px solid var(--border); cursor: pointer; font-variant-numeric: tabular-nums; }
  .row.compact { height: var(--row-h-compact); }
  .row:hover { background: var(--raised); }
  .st, .prio { display: grid; place-items: center; }
  .name { display: flex; align-items: center; gap: var(--s-3); min-width: 0; font-weight: 500; }
  .name a { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: inherit; text-decoration: none; }
  .cell { min-width: 0; display: flex; align-items: center; color: var(--muted); overflow: hidden; }
</style>

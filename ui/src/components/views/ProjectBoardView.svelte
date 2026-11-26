<script lang="ts">
  // The Board layout of a project view: a column per status (or priority, or a select property); dropping a card
  // on another column sets that field, one undo step, like the issue board.
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { SavedView } from '../../lib/bindings';
  import { priorityGlyph } from '../../lib/priority';
  import { groupRows, shownFields, statusGlyph, type Group } from '../../lib/projectViews';
  import Contour from '../../ui/Contour.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import ProjectField from './ProjectField.svelte';
  import { projectData } from './projectData.svelte';

  let { view }: { view: SavedView } = $props();
  const data = projectData(() => view.def);
  const fields = $derived(shownFields(view.def, true));
  // Every column shows, even an empty one: it is where a card can be dropped.
  const columns = $derived(groupRows(data.rows, view.def.group_by, data.ctx, { empty: true, fallback: true }));
  let closed = $state<string[]>([]);
  let over = $state<string | null>(null);
  let error = $state('');

  async function drop(g: Group, e: DragEvent) {
    e.preventDefault();
    over = null;
    const id = Number(e.dataTransfer?.getData('text/plain'));
    const row = data.rows.find((r) => r.project.id === id);
    if (!row || g.rows.some((r) => r.project.id === id)) return;
    error = '';
    try {
      await api.projects.update(g.patch(row.project));
    } catch (err) {
      error = (err as { message?: string }).message ?? String(err);
    }
  }
</script>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if data.loaded && data.rows.length === 0 && view.def.filter}
  <EmptyState title="No projects on this board" hint="Projects that match this view will show up here." />
{:else}
  <div class="board">
    {#each columns as g (g.key)}
      {@const shut = closed.includes(g.key)}
      <section class="column" aria-label={g.name} data-column={g.name} data-over={over === g.key ? '' : undefined}
        ondragover={(e) => (e.preventDefault(), (over = g.key))} ondragleave={() => (over = null)} ondrop={(e) => drop(g, e)}>
        <GroupHeader name={g.name} count={g.rows.length} tone={g.priority ? (Math.min(4, g.priority) as 1 | 2 | 3 | 4) : 1} category={g.status ? statusGlyph(g.status) : undefined} icon={g.status?.icon} color={g.status?.color}
          collapsed={shut} ontoggle={() => (closed = shut ? closed.filter((k) => k !== g.key) : [...closed, g.key])}>
          {#snippet leading()}{#if g.priority !== null}<PriorityGlyph level={priorityGlyph(g.priority)} />{/if}{/snippet}
        </GroupHeader>
        {#if !shut}
          {#if g.rows.length}
            <ul class="cards" aria-label={`${g.name} projects`}>
              {#each g.rows as r (r.project.id)}
                {@const p = r.project}
                <li class="card" class:compact={view.def.density === 'compact'} draggable="true" data-slug={p.slug} data-id={p.id}
                  ondragstart={(e) => { e.dataTransfer?.setData('text/plain', String(p.id)); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
                  ondragend={() => (over = null)} onclick={() => navigate(`/project/${p.slug}`)} role="presentation">
                  <span class="top"><span class="title">{p.name}</span><PriorityGlyph level={priorityGlyph(p.priority)} size={14} /></span>
                  <span class="foot">{#each fields as f (f.Property ?? f.Fixed)}<span class="fv" data-field={f.Property !== undefined ? `P:${f.Property}` : `F:${f.Fixed}`}><ProjectField field={f} row={r} ctx={data.ctx} /></span>{/each}</span>
                </li>
              {/each}
            </ul>
          {:else}
            <div class="none"><Contour size={170} /><p>No projects</p></div>
          {/if}
        {/if}
      </section>
    {/each}
  </div>
{/if}

<style>
  .error { color: var(--signal); font-size: var(--fs-md); margin: var(--s-4) var(--s-6); }
  .board { display: flex; align-items: stretch; gap: var(--s-4); padding: var(--s-4) var(--s-6) var(--s-6); overflow-x: auto; height: 100%; min-height: 0; box-sizing: border-box; }
  .column { flex: 0 0 260px; display: flex; flex-direction: column; min-height: 0; max-height: 100%; background: var(--raised); border-radius: var(--r-pop); outline: 1px solid transparent; overflow: hidden; transition: outline-color var(--dur-1) var(--ease); }
  .column[data-over] { outline-color: var(--focus); }
  .cards { list-style: none; margin: 0; padding: var(--s-4); display: grid; align-content: start; gap: var(--s-3); overflow-y: auto; min-height: 0; }
  .card { display: grid; gap: var(--s-3); padding: var(--s-5) var(--s-6); background: var(--card); border: 1px solid var(--border); border-radius: var(--r-pop); cursor: grab; transition: background var(--dur-1) var(--ease); }
  .card:hover { background: var(--active); }
  .card.compact { padding: var(--s-3) var(--s-5); }
  .top { display: flex; align-items: center; justify-content: space-between; gap: var(--s-4); }
  .title { font-size: var(--fs-md); font-weight: 600; line-height: var(--lh-body); color: var(--ink); }
  .foot { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-3); color: var(--muted); font-size: var(--fs-sm); }
  .fv { display: inline-flex; min-width: 0; }
  .none { display: grid; justify-items: center; gap: var(--s-3); padding: var(--s-7) var(--s-6); color: var(--control); text-align: center; }
  .none p { color: var(--muted); font-size: var(--fs-sm); }
</style>

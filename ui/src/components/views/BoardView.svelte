<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Issue, IssuePatch, Label, Project, SavedView, StatusCategory } from '../../lib/bindings';
  import { selection } from '../../lib/selection.svelte';
  import { issues, propertyDefs, statuses } from '../../lib/stores.svelte';
  import Contour from '../../ui/Contour.svelte';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';

  let { view }: { view: SavedView } = $props();

  // One column per status, or per option of the view's group_by select property (plus "None" for unset values).
  type Column = {
    name: string;
    category: StatusCategory;
    holds: (i: Issue) => boolean;
    patch: (i: Issue) => IssuePatch;
  };

  const groupBy = $derived(view.def.group_by);
  const selectProp = $derived.by(() => {
    const key = groupBy?.Property;
    return propertyDefs.find((d) => d.key === key && d.kind === 'Select') ?? null;
  });

  const columns = $derived.by((): Column[] => {
    const def = selectProp;
    if (def) {
      const options = def.options.map((o) => o.value);
      const bucket = (i: Issue) => {
        const v = i.properties[def.key];
        return typeof v === 'string' && options.includes(v) ? v : null;
      };
      const cols: Column[] = options.map((value) => ({
        name: value,
        category: 'Unstarted',
        holds: (i) => bucket(i) === value,
        patch: (i) => ({ properties: { ...i.properties, [def.key]: value } }),
      }));
      cols.push({
        name: 'None',
        category: 'Unstarted',
        holds: (i) => bucket(i) === null,
        patch: (i) => ({ properties: { ...i.properties, [def.key]: null } }),
      });
      return cols;
    }
    return [...statuses]
      .sort((a, b) => a.position - b.position)
      .map((s) => ({ name: s.name, category: s.category, holds: (i) => i.status_id === s.id, patch: () => ({ status_id: s.id }) }));
  });

  // Per-viewer layout memory. Storage can be blocked (private window), so every access is guarded.
  const read = <T,>(key: string, fallback: T): T => {
    try {
      const raw = localStorage.getItem(key);
      return raw === null ? fallback : (JSON.parse(raw) as T);
    } catch {
      return fallback;
    }
  };
  const write = (key: string, value: unknown) => {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // Not persisted: the layout still works for this session.
    }
  };
  const collapseKey = `board.${view.id}.collapsed`;
  const hideKey = `board.${view.id}.hideEmpty`;

  // Explicit choices only; a column without one is collapsed when its status is Done or Canceled.
  let overrides = $state<Record<string, boolean>>(read(collapseKey, {}));
  let hideEmpty = $state(read(hideKey, false));
  const isCollapsed = (col: Column) => overrides[col.name] ?? (col.category === 'Completed' || col.category === 'Canceled');

  function toggleCollapsed(col: Column) {
    overrides = { ...overrides, [col.name]: !isCollapsed(col) };
    write(collapseKey, overrides);
  }
  function toggleHideEmpty() {
    hideEmpty = !hideEmpty;
    write(hideKey, hideEmpty);
  }

  let labels: Label[] = $state([]);
  let projects: Project[] = $state([]);
  let error: string | null = $state(null);
  const labelOf = (id: number) => labels.find((l) => l.id === id);
  const projectOf = (i: Issue) => projects.find((p) => p.id === i.project_id);

  onMount(async () => {
    [labels, projects] = await Promise.all([api.labels.list(), api.projects.list()]);
  });

  const byColumn = $derived(columns.map((col) => ({ col, cards: issues.filter(col.holds) })));
  const rows = $derived(hideEmpty ? byColumn.filter((r) => r.cards.length > 0) : byColumn);
  // Reading order over the cards a keyboard or shift-click can reach (collapsed columns are skipped).
  const ordered = $derived(rows.flatMap((r) => (isCollapsed(r.col) ? [] : r.cards.map((i) => i.id))));

  let over = $state<string | null>(null);

  async function drop(col: Column, e: DragEvent) {
    e.preventDefault();
    over = null;
    await moveTo(col, Number(e.dataTransfer?.getData('text/plain')));
  }

  function pick(e: MouseEvent, id: number) {
    if (e.shiftKey) {
      const at = selection.focused === null ? -1 : ordered.indexOf(selection.focused);
      if (at < 0) return selection.focus(id);
      return selection.move(ordered, ordered.indexOf(id) - at, true);
    }
    if (e.ctrlKey || e.metaKey) {
      selection.focus(id);
      return selection.toggle(id);
    }
    selection.clear();
    selection.focus(id);
  }

  const isSelected = (id: number) => selection.focused === id || selection.selected.includes(id);

  const due = (i: Issue) => i.due?.Date ?? i.due?.DateTime?.utc.slice(0, 10) ?? null;
  const dueLabel = (d: string) => new Date(`${d}T00:00:00`).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });

  async function moveTo(col: Column, id: number) {
    const at = issues.findIndex((i) => i.id === id);
    const before = issues[at];
    if (!before || col.holds(before)) return;
    const patch = col.patch(before);
    issues[at] = { ...before, ...patch };
    error = null;
    try {
      issues[at] = await api.issues.update(id, patch);
    } catch (err) {
      const now = issues.findIndex((i) => i.id === id);
      if (now >= 0) issues[now] = before;
      error = (err as { message?: string }).message ?? String(err);
    }
  }
</script>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<div class="bar">
  <span class="count">{issues.length} issues</span>
  <div class="grow"></div>
  <IconButton icon="filter" label="Hide empty columns" pressed={hideEmpty} onclick={toggleHideEmpty} />
</div>

<div class="board">
  {#each rows as { col, cards } (col.name)}
    {@const collapsed = isCollapsed(col)}
    <section
      class="column"
      aria-label={col.name}
      data-column={col.name}
      data-collapsed={collapsed ? '' : undefined}
      data-over={over === col.name ? '' : undefined}
      ondragover={(e) => {
        e.preventDefault();
        over = col.name;
      }}
      ondragleave={() => (over = null)}
      ondrop={(e) => drop(col, e)}
    >
      <GroupHeader
        name={col.name}
        count={cards.length}
        category={col.category}
        {collapsed}
        ontoggle={() => toggleCollapsed(col)}
        onadd={() => dispatchEvent(new CustomEvent('command:issue.create'))}
      />
      {#if !collapsed}
        {#if cards.length}
          <ul class="cards" role="listbox" aria-label={`${col.name} issues`} aria-multiselectable="true">
            {#each cards as i (i.id)}
              {@const project = projectOf(i)}
              <li
                class="card"
                role="option"
                tabindex={selection.focused === i.id ? 0 : -1}
                draggable="true"
                data-issue={i.key}
                aria-selected={isSelected(i.id)}
                data-focused={selection.focused === i.id ? '' : undefined}
                onclick={(e) => pick(e, i.id)}
                ondragstart={(e) => {
                  e.dataTransfer?.setData('text/plain', String(i.id));
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
                }}
                ondragend={() => (over = null)}
              >
                <span class="top">
                  <span class="key">{i.key}</span>
                  <PriorityGlyph level={i.priority} size={14} />
                </span>
                <span class="title">{i.title}</span>
                <span class="foot">
                  {#if project}<span class="project"><EntityIcon icon={project.icon} color={project.color} size={12} />{project.name}</span>{/if}
                  {#each i.label_ids as id (id)}
                    {#if labelOf(id)}<Pill color={labelOf(id)?.color}>{labelOf(id)?.name}</Pill>{/if}
                  {/each}
                  {#if due(i)}<time class="due" datetime={due(i) ?? undefined}>{dueLabel(due(i) ?? '')}</time>{/if}
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="none">
            <Contour size={170} />
            <p>No issues</p>
          </div>
        {/if}
      {/if}
    </section>
  {/each}
</div>

<style>
  .error {
    color: var(--signal);
    font-size: var(--fs-md);
    margin: var(--s-4) var(--s-6);
  }
  .bar {
    height: var(--row-h);
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: 0 var(--s-6);
  }
  .grow { flex: 1; }
  .count { color: var(--muted); font-size: var(--fs-sm); }
  .board {
    display: flex;
    align-items: stretch;
    gap: var(--s-4);
    padding: var(--s-4) var(--s-6) var(--s-6);
    overflow-x: auto;
    height: 100%;
    min-height: 0;
  }
  .column {
    flex: 0 0 260px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    max-height: 100%;
    background: var(--raised);
    border-radius: var(--r-pop);
    outline: 1px solid transparent;
    overflow: hidden;
    transition: outline-color var(--dur-1) var(--ease);
  }
  .column[data-over] { outline-color: var(--focus); }
  .column[data-collapsed] { flex-basis: 260px; }
  .cards {
    list-style: none;
    margin: 0;
    padding: var(--s-4);
    display: grid;
    align-content: start;
    gap: var(--s-3);
    overflow-y: auto;
    min-height: 0;
  }
  .card {
    display: grid;
    gap: var(--s-2);
    padding: var(--s-5) var(--s-6);
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-pop);
    cursor: grab;
    transition: background var(--dur-1) var(--ease);
  }
  .card:hover { background: var(--active); }
  .card[aria-selected='true'] { background: var(--selection); }
  .card[data-focused] { outline: 2px solid var(--signal); outline-offset: 1px; }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-4);
  }
  .key {
    color: var(--muted);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .title {
    font-size: var(--fs-md);
    font-weight: 600;
    line-height: var(--lh-body);
    color: var(--ink);
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-3);
    color: var(--muted);
    font-size: var(--fs-sm);
  }
  .project { display: inline-flex; align-items: center; gap: var(--s-2); margin-right: var(--s-2); }
  .due { margin-left: auto; font-variant-numeric: tabular-nums; }
  .none {
    display: grid;
    justify-items: center;
    gap: var(--s-3);
    padding: var(--s-7) var(--s-6);
    color: var(--control);
    text-align: center;
  }
  .none p { color: var(--muted); font-size: var(--fs-sm); }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Issue, IssuePatch, Label, SavedView } from '../../lib/bindings';
  import { issues, propertyDefs, statuses } from '../../lib/stores.svelte';
  import { registerShortcut } from '../../lib/keymap';
  import { navigate } from '../../app/routes';

  let { view }: { view: SavedView } = $props();

  // One column per status, or per option of the view's group_by select property (plus "None" for unset values).
  type Column = { name: string; holds: (i: Issue) => boolean; patch: (i: Issue) => IssuePatch };

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
        holds: (i) => bucket(i) === value,
        patch: (i) => ({ properties: { ...i.properties, [def.key]: value } }),
      }));
      cols.push({
        name: 'None',
        holds: (i) => bucket(i) === null,
        patch: (i) => ({ properties: { ...i.properties, [def.key]: null } }),
      });
      return cols;
    }
    return [...statuses]
      .sort((a, b) => a.position - b.position)
      .map((s) => ({ name: s.name, holds: (i) => i.status_id === s.id, patch: () => ({ status_id: s.id }) }));
  });

  let labels: Label[] = $state([]);
  let error: string | null = $state(null);
  const labelName = (id: number) => labels.find((l) => l.id === id)?.name;

  onMount(async () => {
    labels = await api.labels.list();
  });

  async function drop(col: Column, e: DragEvent) {
    e.preventDefault();
    await moveTo(col, Number(e.dataTransfer?.getData('text/plain')));
  }

  // Cards in reading order (column by column); j/k walk it.
  const ordered = $derived(columns.flatMap((c) => issues.filter(c.holds)));
  let selectedId = $state<number | null>(null);
  const current = $derived(ordered.find((i) => i.id === selectedId) ?? ordered[0] ?? null);
  const step = (d: number) => {
    const at = ordered.findIndex((i) => i.id === current?.id);
    selectedId = ordered[Math.min(Math.max(at + d, 0), ordered.length - 1)]?.id ?? null;
  };
  // s moves the card one column right (wrapping); p cycles its priority.
  const advance = () => {
    const at = columns.findIndex((c) => current && c.holds(current));
    if (current && at >= 0) void moveTo(columns[(at + 1) % columns.length], current.id);
  };
  const bump = async () => {
    if (!current) return;
    const id = current.id;
    try {
      const next = await api.issues.update(id, { priority: (current.priority + 1) % 5 });
      const at = issues.findIndex((i) => i.id === id);
      if (at >= 0) issues[at] = next;
    } catch (err) {
      error = (err as { message?: string }).message ?? String(err);
    }
  };
  onMount(() => {
    const offs = [
      registerShortcut('j', 'board.next', () => step(1)),
      registerShortcut('k', 'board.previous', () => step(-1)),
      registerShortcut('enter', 'board.open', () => current && navigate(`/issue/${current.key}`)),
      registerShortcut('s', 'board.status', advance),
      registerShortcut('p', 'board.priority', () => void bump()),
    ];
    return () => offs.forEach((off) => off());
  });

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

<div class="board">
  {#each columns as col (col.name)}
    {@const cards = issues.filter(col.holds)}
    <section
      class="column"
      aria-label={col.name}
      data-column={col.name}
      ondragover={(e) => e.preventDefault()}
      ondrop={(e) => drop(col, e)}
    >
      <header>
        <span>{col.name}</span>
        <span class="count">{cards.length}</span>
      </header>
      {#each cards as i (i.id)}
        <article
          class="card"
          draggable="true"
          data-issue={i.key}
          aria-selected={i.id === current?.id}
          ondragstart={(e) => {
            e.dataTransfer?.setData('text/plain', String(i.id));
            if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
          }}
        >
          <span class="key">{i.key}</span>
          <span class="title">{i.title}</span>
          <span class="meta">
            {#if i.priority > 0}<span class="priority">P{i.priority}</span>{/if}
            {#each i.label_ids as id (id)}
              {#if labelName(id)}<span class="label">{labelName(id)}</span>{/if}
            {/each}
          </span>
        </article>
      {/each}
    </section>
  {/each}
</div>

<style>
  .error {
    color: #b42318;
    font-size: 13px;
    margin: 8px 12px;
  }
  .board {
    display: flex;
    gap: 8px;
    padding: 12px;
    overflow-x: auto;
    align-items: flex-start;
  }
  .column {
    flex: 0 0 240px;
    min-height: 120px;
    background: #f5f5f5;
    border-radius: 6px;
    padding: 6px;
  }
  .column header {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    font-weight: 600;
    color: #555;
    padding: 4px 6px 8px;
  }
  .count {
    color: #888;
    font-weight: 400;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: #fff;
    border: 1px solid #e3e3e3;
    border-radius: 5px;
    padding: 8px;
    margin-bottom: 6px;
    cursor: grab;
    font-size: 13px;
  }
  .key {
    color: #888;
    font-size: 11px;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    font-size: 11px;
    color: #666;
  }
  .label {
    background: #eee;
    border-radius: 3px;
    padding: 0 4px;
  }
  @media (prefers-color-scheme: dark) {
    .column { background: #1e1e1e; }
    .column header { color: #bbb; }
    .card { background: #2a2a2a; border-color: #3a3a3a; color: #eee; }
    .meta { color: #aaa; }
    .label { background: #3a3a3a; }
  }
</style>

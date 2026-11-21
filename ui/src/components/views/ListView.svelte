<script lang="ts">
  import { api } from '../../lib/api';
  import type { FieldRef, Issue, PropertyDef, SavedView, Status, ViewDef } from '../../lib/bindings';
  import { registerShortcut } from '../../lib/keymap';
  import { navigate } from '../../app/routes';

  const ROW = 32;
  // shortcut: fixed 800px viewport fallback when the list is not laid out (tests), upgrade if rows vary in height
  const FALLBACK_VIEWPORT = 800;
  const OVERSCAN = 10;
  const VIRTUALIZE_ABOVE = 200;
  // Linear's order: no priority first, then urgent to low
  const PRIORITY = ['No priority', 'Urgent', 'High', 'Medium', 'Low'];
  const BARS = [0, 4, 3, 2, 1];

  let { view }: { view: SavedView } = $props();

  type Item = { head: string } | { row: Issue; group: string };

  let issues = $state<Issue[]>([]);
  let statuses = $state<Status[]>([]);
  let defs = $state<PropertyDef[]>([]);
  let loaded = $state(false);
  let selectedId = $state<number | null>(null);
  let picker = $state<'status' | 'priority' | null>(null);
  let scrollTop = $state(0);
  let height = $state(0);
  let scroller = $state<HTMLElement>();

  const def = $derived<ViewDef>(view.def);

  async function load(v: SavedView) {
    const [st, dd, rows] = await Promise.all([api.statuses.list(), api.properties.listDefs(), api.views.query(v.def)]);
    statuses = st;
    defs = dd;
    issues = rows;
    loaded = true;
  }

  $effect(() => {
    load(view);
  });

  // Groups keep the order the view's query returned; status groups follow the status positions.
  const groupOf = (i: Issue): { label: string; order: number } => {
    const field: FieldRef | null = def.group_by;
    if (field?.Fixed === 'Status') {
      const st = statuses.find((s) => s.id === i.status_id);
      return st ? { label: st.name, order: st.position } : { label: 'Unknown', order: Infinity };
    }
    if (field?.Fixed === 'Priority') return { label: PRIORITY[i.priority] ?? 'Unknown', order: i.priority };
    if (field?.Property !== undefined) {
      const v = i.properties[field.Property];
      return { label: v == null ? 'None' : String(v), order: 0 };
    }
    return { label: '', order: 0 };
  };

  const items = $derived.by(() => {
    const groups = new Map<string, { order: number; rows: Issue[] }>();
    for (const i of issues) {
      const g = groupOf(i);
      const entry = groups.get(g.label) ?? { order: g.order, rows: [] };
      entry.rows.push(i);
      groups.set(g.label, entry);
    }
    const sorted = [...groups].sort((a, b) => a[1].order - b[1].order);
    const out: Item[] = [];
    for (const [label, g] of sorted) {
      if (label !== '') out.push({ head: label });
      for (const row of g.rows) out.push({ row, group: label });
    }
    return out;
  });

  const rowItems = $derived(items.filter((x): x is { row: Issue; group: string } => 'row' in x));
  const current = $derived(rowItems.find((x) => x.row.id === selectedId)?.row ?? rowItems[0]?.row ?? null);
  const viewport = $derived(height || FALLBACK_VIEWPORT);
  const virtual = $derived(items.length > VIRTUALIZE_ABOVE);
  const first = $derived(virtual ? Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN) : 0);
  const last = $derived(virtual ? Math.min(items.length, Math.ceil((scrollTop + viewport) / ROW) + OVERSCAN) : items.length);
  const visible = $derived(items.slice(first, last).map((item, k) => ({ item, top: (first + k) * ROW })));

  const due = (i: Issue) => i.due?.Date ?? i.due?.DateTime?.utc.slice(0, 10) ?? '';
  const chips = (i: Issue) =>
    def.columns.flatMap((c) => {
      if (c.Property === undefined || i.properties[c.Property] == null) return [];
      const name = defs.find((d) => d.key === c.Property)?.name ?? c.Property;
      return [{ name, value: String(i.properties[c.Property]) }];
    });

  function measure() {
    height = scroller?.clientHeight ?? 0;
  }

  $effect(() => {
    if (scroller) measure();
  });

  function move(delta: number) {
    if (!rowItems.length) return;
    const at = rowItems.findIndex((x) => x.row.id === current?.id);
    const next = Math.max(0, Math.min(rowItems.length - 1, at + delta));
    selectedId = rowItems[next].row.id;
    const top = items.indexOf(rowItems[next]) * ROW;
    if (!scroller) return;
    if (top < scroller.scrollTop) scroller.scrollTop = top;
    else if (top + ROW > scroller.scrollTop + viewport) scroller.scrollTop = top + ROW - viewport;
  }

  function open() {
    if (current && !picker) navigate(`/issue/${current.key}`);
  }

  async function set(patch: Record<string, number>) {
    if (!current) return;
    await api.issues.update(current.id, patch);
    picker = null;
    await load(view);
  }

  $effect(() => {
    const offs = [
      registerShortcut('j', 'list.next', () => move(1)),
      registerShortcut('k', 'list.previous', () => move(-1)),
      registerShortcut('enter', 'list.open', open),
      registerShortcut('s', 'list.status', () => (picker = current ? 'status' : null)),
      registerShortcut('p', 'list.priority', () => (picker = current ? 'priority' : null)),
      registerShortcut('esc', 'list.close', () => (picker = null)),
    ];
    return () => offs.forEach((off) => off());
  });
</script>

<svelte:window onresize={measure} />

{#if loaded && issues.length === 0}
  <p class="empty">No issues in this view.</p>
{:else}
  {#if picker === 'status'}
    <div class="picker" role="menu" aria-label="Status">
      {#each statuses as st (st.id)}
        <button role="menuitem" onclick={() => set({ status_id: st.id })}>{st.name}</button>
      {/each}
    </div>
  {:else if picker === 'priority'}
    <div class="picker" role="menu" aria-label="Priority">
      {#each PRIORITY as label, p (p)}
        <button role="menuitem" onclick={() => set({ priority: p })}>{label}</button>
      {/each}
    </div>
  {/if}

  <div class="scroller" role="presentation" bind:this={scroller} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
    <ul role="listbox" class="list" style:height="{items.length * ROW}px">
      {#each visible as { item, top } (top)}
        {#if 'head' in item}
          <li role="presentation" class="group-head" style:top="{top}px">{item.head}</li>
        {:else}
          {@const i = item.row}
          <li
            role="option"
            class="row"
            data-key={i.key}
            aria-selected={i.id === current?.id}
            style:top="{top}px"
          >
            <span class="prio" aria-label={PRIORITY[i.priority]} title={PRIORITY[i.priority]}>
              {#each BARS.slice(1) as bar, k (k)}<i class:on={i.priority > 0 && 4 - i.priority >= k}></i>{/each}
            </span>
            <span class="key">{i.key}</span>
            <span class="title">{i.title}</span>
            {#each chips(i) as chip (chip.name)}
              <span class="chip">{chip.name}: {chip.value}</span>
            {/each}
            <span class="status" style:color={statuses.find((s) => s.id === i.status_id)?.color}>
              {statuses.find((s) => s.id === i.status_id)?.name ?? 'Unknown'}
            </span>
            <span class="due">{due(i)}</span>
          </li>
        {/if}
      {/each}
    </ul>
  </div>
{/if}

<style>
  .scroller { overflow-y: auto; max-height: 100%; height: 100%; position: relative; font-size: 13px; }
  .list { list-style: none; margin: 0; padding: 0; position: relative; }
  .group-head, .row { position: absolute; left: 0; right: 0; height: 32px; box-sizing: border-box; display: flex; align-items: center; gap: 10px; padding: 0 16px; }
  .group-head { font-weight: 600; color: #666; text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
  .row { border-bottom: 1px solid #eee; }
  .row[aria-selected='true'] { background: #eef3ff; }
  .key { color: #888; font-variant-numeric: tabular-nums; min-width: 64px; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chip { border: 1px solid #ddd; border-radius: 4px; padding: 0 6px; color: #555; }
  .status, .due { color: #666; min-width: 90px; }
  .prio { display: inline-flex; gap: 2px; width: 22px; }
  .prio i { width: 3px; height: 10px; background: #ccc; border-radius: 1px; }
  .prio i.on { background: #444; }
  .picker { display: flex; flex-wrap: wrap; gap: 6px; padding: 8px 16px; border-bottom: 1px solid #ddd; }
  .empty { padding: 24px 16px; color: #666; }
</style>

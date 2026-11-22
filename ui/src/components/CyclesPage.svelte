<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Cycle, Issue, SavedView, Status } from '../lib/bindings';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import GroupHeader from '../ui/GroupHeader.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import ListView from './views/ListView.svelte';

  type Band = 'current' | 'upcoming' | 'past';
  type Row = Cycle & { band: Band; done: number; total: number };

  const DAY = 86_400_000;
  const CATEGORY: Record<Band, 'Started' | 'Unstarted' | 'Completed'> = { current: 'Started', upcoming: 'Unstarted', past: 'Completed' };
  const LABEL: Record<Band, string> = { current: 'Current', upcoming: 'Upcoming', past: 'Past' };

  let rows = $state<Row[]>([]);
  let statuses = $state<Status[]>([]);
  let loaded = $state(false);
  let opened = $state<number | null>(null);
  let collapsed = $state<Record<Band, boolean>>({ current: false, upcoming: false, past: true });
  let creating = $state(false);
  let moving = $state(false);
  let name = $state('');
  let start = $state('');
  let end = $state('');

  // Local calendar day as YYYY-MM-DD; dates in the store are day strings, so they compare as text.
  const isoDay = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  const bandOf = (c: Cycle, today: string): Band => (c.start_date > today ? 'upcoming' : c.end_date < today ? 'past' : 'current');
  const title = (c: Cycle) => c.name ?? `Cycle ${c.number}`;
  const cycleView = (c: Cycle): SavedView => ({
    id: 0,
    name: title(c),
    position: 0,
    icon: null,
    color: null,
    def: { filter: { Cond: { field: { Fixed: 'Cycle' }, op: 'Eq', value: c.id } }, sorts: [], group_by: { Fixed: 'Status' }, layout: 'List', columns: [], include_archived: false },
  });
  const finished = (i: Issue) => ['Completed', 'Canceled'].includes(statuses.find((s) => s.id === i.status_id)?.category ?? '');

  async function load() {
    const today = isoDay(new Date());
    const list = await api.cycles.list();
    const issues = await Promise.all(list.map((c) => api.views.query(cycleView(c).def)));
    rows = list
      .map((c, i) => ({ ...c, band: bandOf(c, today), done: issues[i].filter(finished).length, total: issues[i].length }))
      .sort((a, b) => a.start_date.localeCompare(b.start_date));
    statuses = await api.statuses.list();
    loaded = true;
  }

  const bands = $derived((['current', 'upcoming', 'past'] as Band[]).map((b) => ({ band: b, items: rows.filter((r) => r.band === b) })));
  const current = $derived(rows.find((r) => r.band === 'current'));
  const next = $derived(rows.find((r) => r.band === 'upcoming'));
  const daysLeft = (c: Cycle) => Math.max(0, Math.ceil((Date.parse(c.end_date) - Date.parse(isoDay(new Date()))) / DAY));

  // shortcut: one update per issue, since the backend has no bulk update yet; upgrade to issues_bulk_update(ids, patch) when it lands.
  async function moveUnfinished(from: Cycle, to: Cycle) {
    const issues = await api.views.query(cycleView(from).def);
    for (const i of issues.filter((x) => !finished(x))) await api.issues.update(i.id, { cycle_id: to.id });
    await load();
  }

  async function confirmMove() {
    if (current && next) await moveUnfinished(current, next);
    moving = false;
  }

  async function create() {
    if (!start || !end) return;
    await api.cycles.create(name.trim() || null, start, end, null, null);
    name = '';
    start = '';
    end = '';
    creating = false;
    await load();
  }

  onMount(load);
</script>

<PageHeader title="Cycles" icon="icon:cycle">
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New cycle</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && rows.length === 0}
    <EmptyState title="No cycles yet" hint="A cycle is a dated slice of work. Start one to see what is due this week." />
  {:else}
    {#each bands as group (group.band)}
      {#if group.items.length > 0}
        <section data-group={group.band} class="group">
          <GroupHeader name={LABEL[group.band]} count={group.items.length} category={CATEGORY[group.band]} collapsed={collapsed[group.band]} ontoggle={() => (collapsed[group.band] = !collapsed[group.band])} />
          {#if !collapsed[group.band]}
            <ul class="rows">
              {#each group.items as c (c.id)}
                <li class="row" data-cycle={c.id} class:open={opened === c.id}>
                  <button class="pick" type="button" aria-expanded={opened === c.id} onclick={() => (opened = opened === c.id ? null : c.id)}>
                    <span class="title">{title(c)}</span>
                    <span class="dates">{c.start_date} to {c.end_date}</span>
                    <span class="progress">{c.done} of {c.total} done</span>
                    {#if group.band === 'current'}<span class="left">{daysLeft(c)} days left</span>{/if}
                  </button>
                  {#if group.band === 'current' && next}
                    <Button onclick={() => (moving = true)}>Move unfinished to next cycle</Button>
                  {/if}
                </li>
                {#if opened === c.id}
                  {#key c.id}<div class="issues"><ListView view={cycleView(c)} /></div>{/key}
                {/if}
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    {/each}
  {/if}
</main>

<Dialog bind:open={creating} title="New cycle">
  {#snippet children()}
    <TextInput bind:value={name} label="Cycle name" placeholder="Name (optional)" />
    <TextInput bind:value={start} label="Start date" type="date" />
    <TextInput bind:value={end} label="End date" type="date" />
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (creating = false)}>Cancel</Button>
    <Button variant="primary" onclick={create}>Create cycle</Button>
  {/snippet}
</Dialog>

<Dialog bind:open={moving} title="Move unfinished issues" description={current && next ? `Open issues in ${title(current)} move to ${title(next)}.` : undefined}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (moving = false)}>Cancel</Button>
    <Button variant="primary" onclick={confirmMove}>Move</Button>
  {/snippet}
</Dialog>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); }
  .group { display: grid; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: var(--s-5); min-height: var(--row-h); padding-right: var(--s-4); border-bottom: 1px solid var(--border); }
  .pick { flex: 1; display: flex; align-items: center; gap: var(--s-6); min-height: var(--row-h); padding: 0 var(--s-6); border: 0; background: transparent; color: inherit; text-align: left; }
  .pick:hover { background: var(--active); }
  .title { font-weight: 600; }
  .dates, .progress, .left { color: var(--muted); font-variant-numeric: tabular-nums; }
  .left { margin-left: auto; color: var(--ink); }
  .issues { border-left: 2px solid var(--accent); }
</style>

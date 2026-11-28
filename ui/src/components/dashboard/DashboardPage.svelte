<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Cycle, Label, Project, StaleIssue } from '../../lib/bindings';
  import { localDay } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import Popover from '../../ui/Popover.svelte';
  import Select from '../../ui/Select.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import Tabs from '../../ui/Tabs.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { workspaceZone } from '../views/dates';
  import { CARDS } from './cards';
  import { dashboard, moveCard, orderedCards, set, toggleCard } from './dashboard.svelte';
  import { PRESETS, bucketFor, filterOf, metricsFor, rangeOf, type Metrics } from './metrics';

  let today = $state('');
  let weekStart = $state<'sunday' | 'monday'>('sunday');
  let cycles = $state<Cycle[]>([]);
  let projects = $state<Project[]>([]);
  let labels = $state<Label[]>([]);
  let metrics = $state<Metrics | null>(null);
  let stale = $state<StaleIssue[]>([]);
  let error = $state('');
  let ready = $state(false);

  onMount(async () => {
    const [tz, w, c, p, l] = await Promise.all([workspaceZone(), api.settings.get('week_start').catch(() => null), api.cycles.list(), api.projects.list(), api.labels.list()]);
    [today, weekStart, cycles, projects, labels] = [localDay(new Date(), tz), w === 'monday' ? 'monday' : 'sunday', c, p, l];
    ready = true;
  });

  const range = $derived(ready ? rangeOf(dashboard.range, { today, weekStart, cycles }, dashboard.from, dashboard.to) : null);
  const all = $derived([...PRESETS.filter((p) => p.id !== 'custom'), ...cycles.map((c) => ({ id: `cycle:${c.id}`, label: c.name ?? `Cycle ${c.number}` })), PRESETS.at(-1)!].map((p) => ({ value: p.id, label: p.label })));
  const shown = $derived(orderedCards().filter((id) => !dashboard.hidden.includes(id)).map((id) => CARDS.find((c) => c.id === id)!));

  // A stale answer (the range changed while it was loading) is dropped.
  let asked = 0;
  $effect(() => {
    if (!range) return void (metrics = null);
    const n = ++asked;
    const q = { scope: range.scope, bucket: bucketFor(range.from, range.to), filter: filterOf(dashboard.project, dashboard.label), measure: dashboard.measure };
    error = '';
    metrics = null;
    Promise.all([metricsFor(q), api.stale.list()]).then(
      ([m, st]) => {
        if (n === asked) [metrics, stale] = [m, st];
      },
      (e) => n === asked && (error = (e as { message?: string }).message ?? String(e)),
    );
  });
  const empty = $derived(metrics !== null && metrics.buckets.every((b) => b.created === 0 && b.completed === 0 && b.scope === 0));
  const items = (list: { id: number; name: string }[], none: string) => [{ value: '', label: none }, ...list.map((x) => ({ value: String(x.id), label: x.name }))];
</script>

<PageHeader title="Dashboard" icon="icon:chart">
  {#snippet filters()}
    <Select label="Range" items={all} bind:value={() => dashboard.range, (v) => set({ range: v })} />
    {#if dashboard.range === 'custom'}
      <TextInput type="date" label="From" bind:value={() => dashboard.from, (v) => set({ from: v })} />
      <TextInput type="date" label="To" bind:value={() => dashboard.to, (v) => set({ to: v })} />
    {/if}
    <Select label="Project" items={items(projects, 'All projects')} bind:value={() => String(dashboard.project ?? ''), (v) => set({ project: v ? Number(v) : null })} />
    <Select label="Label" items={items(labels, 'All labels')} bind:value={() => String(dashboard.label ?? ''), (v) => set({ label: v ? Number(v) : null })} />
    <Tabs segmented label="Measure" tabs={[{ id: 'Count', label: 'Count' }, { id: 'Points', label: 'Points' }]} bind:value={() => dashboard.measure, (v) => set({ measure: v as 'Count' | 'Points' })} />
  {/snippet}
  {#snippet actions()}
    <Popover>
      {#snippet trigger(props)}<Button icon="eye" {...props}>Cards</Button>{/snippet}
      <ul class="cards" aria-label="Dashboard cards">
        {#each orderedCards() as id, i (id)}
          <li>
            <Checkbox label={CARDS.find((c) => c.id === id)!.title} checked={!dashboard.hidden.includes(id)} onchange={() => toggleCard(id)} />
            <IconButton icon="arrow-up" label="Move earlier" disabled={i === 0} onclick={() => moveCard(id, -1)} />
            <IconButton icon="arrow-down" label="Move later" disabled={i === CARDS.length - 1} onclick={() => moveCard(id, 1)} />
          </li>
        {/each}
      </ul>
    </Popover>
  {/snippet}
</PageHeader>

<main class="body">
  {#if ready && !range}
    <EmptyState title={dashboard.range === 'custom' ? 'Pick two dates' : 'No cycle runs today'} hint={dashboard.range === 'custom' ? 'Choose a start and an end day for the custom range.' : 'Choose another range, or create a cycle.'} />
  {:else if error}
    <p role="alert" class="error">{error}</p>
  {:else if metrics === null}
    <div class="grid" aria-busy="true" aria-label="Loading the dashboard"><Skeleton height="var(--s-10)" /><Skeleton height="var(--s-10)" /></div>
  {:else if empty}
    <EmptyState title="Nothing in this range" hint="No issue was created or finished in these days. Try a longer range or clear the project and label." />
  {:else if shown.length === 0}
    <EmptyState title="All cards are hidden" hint="Use Cards at the top right to show some." />
  {:else}
    <div class="grid">
      {#each shown as c (c.id)}
        <section class="card" class:wide={c.wide} aria-label={c.title}>
          <h2>{c.title}</h2>
          <c.component {metrics} measure={dashboard.measure} filter={filterOf(dashboard.project, dashboard.label)} cycle={range && 'Cycle' in range.scope ? range.scope.Cycle : null} {stale} />
        </section>
      {/each}
    </div>
  {/if}
</main>

<style>
  .body { padding: var(--s-5) var(--s-7) var(--s-7); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: var(--s-6); }
  .card { padding: var(--s-6); border: 1px solid var(--border); border-radius: var(--r-sheet); background: var(--card); min-width: 0; }
  .card.wide { grid-column: 1 / -1; }
  h2 { margin: 0 0 var(--s-5); font-size: var(--fs-lg); font-weight: 700; letter-spacing: -0.01em; }
  .error { color: var(--c-red); }
  .cards { list-style: none; margin: 0; padding: var(--s-3); display: grid; gap: var(--s-2); }
  .cards li { display: flex; align-items: center; gap: var(--s-2); }
  .cards li :global(:first-child) { flex: 1; }
</style>

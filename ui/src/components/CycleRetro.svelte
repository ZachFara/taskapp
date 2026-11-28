<script lang="ts">
  import { api } from '../lib/api';
  import type { Cycle } from '../lib/bindings';
  import TextArea from '../ui/TextArea.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { CARDS } from './dashboard/cards';
  import { dashboard } from './dashboard/dashboard.svelte';
  import { bucketFor, metricsFor, type Metrics } from './dashboard/metrics';

  // The retrospective of one cycle: what we aimed for, how it went, and the cycle's own numbers from the metrics engine.
  let { cycle }: { cycle: Cycle } = $props();

  // The dashboard cards that answer a retrospective: throughput, say/do and scope change, AI vs me, flow, blocked.
  const SHOWN = ['throughput', 'cycle', 'ai-vs-me', 'flow', 'blocked'];

  let goal = $state(cycle.goal ?? '');
  let notes = $state(cycle.notes ?? '');
  let error = $state('');
  let metrics = $state<Metrics | null>(null);

  // Writes the whole cycle, as the backend takes it; the page reloads on `rows:changed` and passes the saved cycle back.
  async function save(patch: Partial<Cycle>) {
    error = '';
    try {
      await api.cycles.update({ ...cycle, ...patch });
      dispatchEvent(new CustomEvent('rows:changed'));
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  $effect(() => {
    const [id, from, to, measure] = [cycle.id, cycle.start_date, cycle.end_date, dashboard.measure];
    metrics = null;
    let live = true;
    metricsFor({ scope: { Cycle: id }, bucket: bucketFor(from, to), filter: null, measure }).then((m) => live && (metrics = m), () => {});
    return () => (live = false);
  });
</script>

<section class="retro" aria-label="Retrospective">
  <h3>Retrospective</h3>
  <div class="fields">
    <TextInput label="Cycle goal" placeholder="One sentence, set at planning" bind:value={goal} onchange={(v) => save({ goal: v.trim() || null })} />
    <div class="pair">
      <span class="k">Goal met</span>
      <Tabs segmented label="Goal met" tabs={[{ id: 'yes', label: 'Yes' }, { id: 'no', label: 'No' }]}
        bind:value={() => (cycle.goal_met == null ? '' : cycle.goal_met ? 'yes' : 'no'), (v) => save({ goal_met: v === 'yes' })} />
      <span class="k">How it went</span>
      <Tabs segmented label="Rating" tabs={[1, 2, 3, 4, 5].map((n) => ({ id: String(n), label: String(n) }))}
        bind:value={() => String(cycle.rating ?? ''), (v) => save({ rating: Number(v) })} />
    </div>
    <div class="notes" onfocusout={() => notes !== (cycle.notes ?? '') && save({ notes })}>
      <TextArea label="Retrospective notes" placeholder="What went well, what to change (markdown)" rows={4} bind:value={notes} />
    </div>
    {#if error}<p role="alert" class="error">{error}</p>{/if}
  </div>
  {#if metrics}
    <div class="grid">
      {#each CARDS.filter((c) => SHOWN.includes(c.id)) as c (c.id)}
        <section class="card" aria-label={c.title}>
          <h4>{c.title}</h4>
          <c.component {metrics} measure={dashboard.measure} filter={null} cycle={cycle.id} stale={[]} />
        </section>
      {/each}
    </div>
  {/if}
</section>

<style>
  .retro { padding: var(--s-5) var(--s-7) var(--s-7); display: grid; gap: var(--s-5); border-left: 2px solid var(--accent); }
  h3 { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin: 0; }
  h4 { margin: 0 0 var(--s-4); font-size: var(--fs-md); font-weight: 700; }
  .fields { display: grid; gap: var(--s-4); max-width: 640px; }
  .pair { display: flex; align-items: center; gap: var(--s-4); flex-wrap: wrap; }
  .k { color: var(--muted); font-size: var(--fs-sm); }
  .error { margin: 0; color: var(--c-red); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: var(--s-5); }
  .card { padding: var(--s-5); border: 1px solid var(--border); border-radius: var(--r-sheet); background: var(--card); min-width: 0; }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import BarChart from '../charts/BarChart.svelte';
  import ProgressChart from '../charts/ProgressChart.svelte';
  import type { Day } from '../charts/geometry';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import { loadConfig, saveConfig, type Config } from './config';
  import { formatFor, measuresFor, readEstimation, unitOf } from './estimation';
  import { insightsApi, type Measure, type Period, type Scope } from './insightsApi';
  import { bounds, today, trim } from './series';

  // The right-hand panel: headline numbers, one chart, and three choices that are remembered per view, project or
  // cycle. The host decides when it is open; it is closed by default.
  let { kind, id, title, onclose, ideal = false }: {
    kind: 'view' | 'project' | 'cycle';
    id: number;
    title: string;
    onclose: () => void;
    ideal?: boolean;
  } = $props();

  const measureLabel: Record<Measure, string> = { points: 'Points', count: 'Issues', time: 'Time' };
  const chartItems = [
    { value: 'burnup', label: 'Burn-up line' }, { value: 'stacked', label: 'Stacked area' }, { value: 'bars', label: 'Bars per week' },
  ];
  const rangeItems = $derived([
    { value: 'auto', label: kind === 'cycle' ? 'This cycle' : kind === 'project' ? 'Project dates' : '30 days' },
    ...(kind === 'view' ? [] : [{ value: '30', label: '30 days' }]),
    { value: '90', label: '90 days' },
    { value: 'all', label: 'All' },
  ]);

  let allowed = $state<Measure[]>(['count']);
  let cfg = $state<Config | null>(null);
  let days = $state<Day[] | null>(null);
  let periods = $state<Period[]>([]);
  let now = $state('');
  let width = $state<number | null>(null);
  let seq = 0;

  const scope = $derived({ [kind === 'view' ? 'View' : kind === 'project' ? 'Project' : 'Cycle']: id } as Scope);

  onMount(async () => {
    allowed = measuresFor((await readEstimation()).estimation);
    cfg = await loadConfig(kind, id, allowed);
    now = await today();
  });
  // One effect for both queries: a change of config (or of the entity) refetches, and a late answer is dropped.
  $effect(() => {
    if (!cfg || !now) return;
    const { measure, range } = cfg;
    const mine = ++seq;
    days = null;
    void Promise.all([insightsApi.progress(scope, measure, bounds(range, now).from, bounds(range, now).to), insightsApi.velocity(scope, measure, 8)]).then(([p, v]) => {
      if (mine !== seq) return;
      days = p ? trim(p.days) : [];
      periods = v;
    });
  });

  function change(patch: Partial<Config>) {
    cfg = { ...cfg!, ...patch };
    void saveConfig(kind, id, cfg);
  }

  const m = $derived(cfg?.measure ?? 'count');
  const fmt = $derived(formatFor(m));
  const last = $derived.by(() => {
    const live = (days ?? []).filter((d) => d.date <= (now || '9999'));
    return live.at(-1) ?? days?.at(-1) ?? null;
  });
  const pct = $derived(last && last.scope ? Math.round((last.completed / last.scope) * 100) : 0);
  const bars = $derived(periods.map((p) => ({ label: p.label, value: p.completed })));

  // Dragging the left edge resizes; there is no key binding for it, so the panel keeps its width for the session only.
  function grab(e: PointerEvent) {
    const start = e.clientX;
    const from = (e.currentTarget as HTMLElement).parentElement!.getBoundingClientRect().width;
    const move = (ev: PointerEvent) => (width = Math.min(640, Math.max(300, from + start - ev.clientX)));
    const up = () => (removeEventListener('pointermove', move), removeEventListener('pointerup', up));
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
  }
</script>

<aside class="panel" aria-label="Insights" style:width={width ? `${width}px` : undefined}>
  <div class="grip" role="presentation" onpointerdown={grab}></div>
  <header>
    <h2>{title}</h2>
    <IconButton icon="close" label="Close insights" shortcut="shift+i" onclick={onclose} />
  </header>

  {#if cfg}
    <div class="config">
      <span>Measure</span><Select label="Measure" items={allowed.map((v) => ({ value: v, label: measureLabel[v] }))} bind:value={() => cfg!.measure, (v) => change({ measure: v as Measure })} />
      <span>Chart</span><Select label="Chart" items={chartItems} bind:value={() => cfg!.chart, (v) => change({ chart: v as Config["chart"] })} />
      <span>Range</span><Select label="Range" items={rangeItems} bind:value={() => cfg!.range, (v) => change({ range: v as Config["range"] })} />
    </div>
  {/if}

  {#if last}
    <dl class="headline" data-headline>
      {#each [[fmt(last.scope), 'Scope'], [fmt(last.started), 'Started'], [fmt(last.completed), 'Completed'], [`${pct}%`, 'Done']] as [n, label] (label)}
        <div><dd>{n}</dd> <dt>{label}</dt></div>
      {/each}
    </dl>
  {/if}

  <section class="chart">
    {#if cfg?.chart === 'bars'}
      <BarChart {bars} unit={unitOf(m)} title="Completed per week" format={fmt} />
    {:else}
      <ProgressChart {days} measure={m} unit={unitOf(m)} kind={cfg?.chart === 'stacked' ? 'stacked' : 'burnup'} today={now} {ideal} format={fmt} />
    {/if}
  </section>

  {#if cfg?.chart !== 'bars' && periods.length >= 2}
    <section class="chart">
      <h3>Completed per period</h3>
      <BarChart {bars} unit={unitOf(m)} title="Completed per period" format={fmt} />
    </section>
  {/if}
</aside>

<style>
  .panel { position: relative; flex: none; width: var(--panel-w); display: flex; flex-direction: column; gap: var(--s-7); overflow-y: auto; padding: var(--s-7); border-left: 1px solid var(--border); background: var(--sheet); }
  .grip { position: absolute; inset: 0 auto 0 0; width: var(--s-3); cursor: col-resize; }
  header { display: flex; align-items: center; justify-content: space-between; gap: var(--s-4); }
  h2 { font-size: var(--fs-lg); font-weight: 700; }
  h3 { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin-bottom: var(--s-3); }
  .config { display: grid; grid-template-columns: auto 1fr; align-items: center; gap: var(--s-4) var(--s-6); color: var(--muted); font-size: var(--fs-sm); }
  .headline { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--s-4); margin: 0; }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  dt { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  .chart { min-width: 0; }
</style>

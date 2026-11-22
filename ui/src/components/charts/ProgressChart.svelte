<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../ui/Button.svelte';
  import ChartTable from './ChartTable.svelte';
  import EmptyChart from './EmptyChart.svelte';
  import { fit } from './fit';
  import { area, band, line, niceTicks, scales, spread, step, summary, todayIndex, xLabels, type Day } from './geometry';

  // Burn-up: the scope as a stepped ink line, what has started as a lighter line, what is done as a line over a
  // faint area. `kind="stacked"` shows the same three numbers as bands. `days` is null while loading.
  let {
    days,
    measure,
    unit,
    kind = 'burnup',
    today,
    ideal = false,
    compact = false,
    format = (n: number) => String(Math.round(n * 10) / 10),
  }: {
    days: Day[] | null;
    measure: string;
    unit: string;
    kind?: 'burnup' | 'stacked';
    today?: string;
    ideal?: boolean;
    compact?: boolean;
    format?: (n: number) => string;
  } = $props();

  let width = $state(0);
  let hover = $state<number | null>(null);
  let table = $state(false);

  const w = $derived(width || 480);
  const h = $derived(compact ? 120 : 190);
  const box = $derived({ w, h, left: 32, right: compact ? 60 : 76, top: 14, bottom: 22 });
  const list = $derived(days ?? []);
  const empty = $derived(list.length < 2 || list.every((d) => d.scope === 0));
  const ticks = $derived(niceTicks(Math.max(0, ...list.map((d) => d.scope)), compact ? 3 : 4));
  const top = $derived(ticks.at(-1)!);
  const s = $derived(scales(list.length, top, box));
  const base = $derived(s.y(0));
  const mark = $derived(todayIndex(list, today));
  // Only what has happened is drawn after the fact: after today there is no data, so the actual lines stop.
  const upto = $derived(mark === null ? list.length : mark + 1);
  const pts = (pick: (d: Day) => number, n = list.length) => list.slice(0, n).map((d, i) => [s.x(i), s.y(pick(d))] as [number, number]);
  const scope = $derived(pts((d) => d.scope));
  const started = $derived(pts((d) => d.started, upto));
  const done = $derived(pts((d) => d.completed, upto));
  const labels = $derived(xLabels(list.length));

  const end = $derived(list[upto - 1]);
  const marks = $derived.by(() => {
    if (!end) return [];
    const raw = [
      { text: `Scope ${format(list.at(-1)!.scope)}`, y: s.y(list.at(-1)!.scope), x: s.x(list.length - 1), tone: 'scope' },
      ...(compact ? [] : [{ text: `Started ${format(end.started)}`, y: s.y(end.started), x: s.x(upto - 1), tone: 'started' }]),
      { text: `Done ${format(end.completed)}`, y: s.y(end.completed), x: s.x(upto - 1), tone: 'done' },
    ];
    const ys = spread(raw.map((m) => m.y + 4), 13);
    return raw.map((m, i) => ({ ...m, y: ys[i] }));
  });
  const idealEnd = $derived(list.length ? list.at(-1)!.scope : 0);

  function move(e: PointerEvent) {
    const r = (e.currentTarget as Element).getBoundingClientRect();
    const frac = (e.clientX - r.left - box.left) / (w - box.left - box.right);
    hover = Math.min(list.length - 1, Math.max(0, Math.round(frac * (list.length - 1))));
  }
  const tip = $derived(hover === null ? null : list[hover]);
  const said = $derived(tip ? `${tip.date.slice(5)}: scope ${format(tip.scope)}, started ${format(tip.started)}, done ${format(tip.completed)}` : '');

  // The arrow-key commands (defs/insights/chart.day.*) say which way to go; only the chart that has focus moves.
  let svg = $state<SVGSVGElement>();
  onMount(() => {
    const go = (e: Event) => {
      if (!svg?.contains(document.activeElement)) return;
      const by = (e as CustomEvent<number>).detail;
      hover = hover === null ? list.length - 1 : Math.min(list.length - 1, Math.max(0, hover + by));
    };
    addEventListener('command:chart.day', go);
    return () => removeEventListener('command:chart.day', go);
  });
</script>

{#if days === null || empty}
  <EmptyChart loading={days === null} height={compact ? 64 : 120} />
{:else}
  <figure use:fit={(w) => (width = w)}>
    <div class="plot">
      <svg bind:this={svg} data-chart tabindex="0" width={w} height={h} viewBox={`0 0 ${w} ${h}`} role="img" aria-label={summary(list, unit)} onblur={() => (hover = null)}>
        {#each ticks as t, i (t)}
          <line class={i === 0 ? 'base' : 'grid'} x1={box.left} x2={w - box.right} y1={s.y(t)} y2={s.y(t)} />
          <text class="tick" x={box.left - 6} y={s.y(t) + 4} text-anchor="end">{format(t)}</text>
        {/each}
        {#each labels as i (i)}
          <text class="tick" x={s.x(i)} y={h - 5} text-anchor={i === 0 ? 'start' : i === list.length - 1 ? 'end' : 'middle'}>{list[i].date.slice(5)}</text>
        {/each}

        {#if kind === 'stacked'}
          <path class="band b-done" d={band(done, list.slice(0, upto).map((_, i) => [s.x(i), base]))} />
          <path class="band b-started" d={band(started, done)} />
          <path class="band b-rest" d={band(scope.slice(0, upto), started)} />
        {:else}
          {#if ideal}
            <line class="ideal" x1={s.x(0)} y1={base} x2={s.x(list.length - 1)} y2={s.y(idealEnd)} />
            <text class="tick" x={s.x(Math.round((list.length - 1) * 0.6))} y={s.y(idealEnd * 0.6) - 6} text-anchor="middle">Ideal</text>
          {/if}
          <path class="scope" d={step(scope)} />
          <path class="completed-area" d={area(done, base)} />
          <path class="started" d={line(started)} />
          <path class="completed" d={line(done)} />
        {/if}

        {#if mark !== null}
          <line class="today" x1={s.x(mark)} x2={s.x(mark)} y1={box.top - 4} y2={base} />
          <text class="today-label" x={s.x(mark) + 4} y={box.top + 4}>Today</text>
        {/if}
        {#if hover !== null}<line class="guide" x1={s.x(hover)} x2={s.x(hover)} y1={box.top} y2={base} />{/if}
        {#each marks as m (m.tone)}<text class={`end-label ${m.tone}`} x={m.x + 6} y={m.y}>{m.text}</text>{/each}
        <rect class="hit" x={box.left} y="0" width={w - box.left - box.right} height={h} role="presentation" onpointermove={move} onpointerleave={() => (hover = null)} />
      </svg>
      {#if tip && hover !== null}
        <div class="tip" aria-hidden="true" style:left={`${Math.min(s.x(hover), w - 190)}px`}>{said}</div>
      {/if}
      <div class="said" role="status">{said}</div>
    </div>
    {#if !compact}
      <div class="more"><Button aria-pressed={table} onclick={() => (table = !table)}>{table ? 'Hide table' : 'View as table'}</Button></div>
      {#if table}
        <ChartTable caption={`Progress in ${unit}`} head={['Day', 'Scope', 'Started', 'Completed']} rows={list.map((d) => [d.date, format(d.scope), format(d.started), format(d.completed)])} />
      {/if}
    {/if}
  </figure>
{/if}

<style>
  figure { margin: 0; display: grid; gap: var(--s-4); min-width: 0; }
  .plot { position: relative; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--border); stroke-width: 1; }
  .base { stroke: var(--control); stroke-width: 1.5; }
  .tick { fill: var(--muted); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .scope { fill: none; stroke: var(--ink); stroke-width: 2; stroke-linejoin: round; }
  .completed { fill: none; stroke: var(--accent); stroke-width: 2; stroke-linejoin: round; }
  .completed-area { fill: var(--accent); fill-opacity: 0.14; stroke: none; }
  .started { fill: none; stroke: var(--accent); stroke-opacity: 0.45; stroke-width: 1.5; stroke-linejoin: round; }
  .ideal { stroke: var(--muted); stroke-width: 1; stroke-dasharray: 4 4; }
  .today { stroke: var(--signal); stroke-width: 1; }
  .today-label { stroke: var(--chart-bg, var(--sheet)); stroke-width: 3px; paint-order: stroke; fill: var(--signal); font-size: var(--fs-xs); font-weight: 600; }
  .guide { stroke: var(--control); stroke-width: 1; }
  .band { stroke: var(--sheet); stroke-width: 1.5; }
  .b-done { fill: var(--accent); }
  .b-started { fill: var(--accent); fill-opacity: 0.4; }
  .b-rest { fill: var(--active); }
  .end-label { stroke: var(--chart-bg, var(--sheet)); stroke-width: 3px; paint-order: stroke; font-size: var(--fs-xs); font-weight: 600; font-variant-numeric: tabular-nums; fill: var(--muted); }
  .end-label.scope { fill: var(--ink); }
  .end-label.done { fill: var(--accent); }
  .hit { fill: transparent; }
  .tip { position: absolute; top: 0; padding: var(--s-2) var(--s-4); background: var(--card); border: 1px solid var(--border); border-radius: var(--r-sm); color: var(--ink); font-size: var(--fs-xs); font-variant-numeric: tabular-nums; pointer-events: none; white-space: nowrap; }
  .more { display: flex; }
  svg:focus-visible { outline: 2px solid var(--focus, var(--accent)); outline-offset: 2px; }
  .said { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
</style>

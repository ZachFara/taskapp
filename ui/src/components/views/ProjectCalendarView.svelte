<script lang="ts">
  // The Calendar layout of a project view: a month grid with each project's start and target day as chips.
  import { onMount } from 'svelte';
  import { navigate } from '../../app/routes';
  import type { SavedView } from '../../lib/bindings';
  import { shortcutFor } from '../../lib/commands/registry';
  import { getWeekStart } from '../../lib/theme.svelte';
  import { dateChips, monthDays } from '../../lib/projectViews';
  import { localDay } from '../../lib/gantt';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import { projectData } from './projectData.svelte';

  let { view, today = localDay(new Date(), Intl.DateTimeFormat().resolvedOptions().timeZone) }: { view: SavedView; today?: string } = $props();
  const data = projectData(() => view.def);
  let shown = $state({ year: Number(today.slice(0, 4)), month: Number(today.slice(5, 7)) - 1 });
  let monday = $state(false);
  const days = $derived(monthDays(shown.year, shown.month, monday));
  const chips = $derived(dateChips(data.rows));
  const title = $derived(new Date(Date.UTC(shown.year, shown.month, 1)).toLocaleDateString('en-US', { month: 'long', year: 'numeric', timeZone: 'UTC' }));
  const weekdays = $derived((monday ? ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] : ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']));

  function step(by: number) {
    const d = new Date(Date.UTC(shown.year, shown.month + by, 1));
    shown = { year: d.getUTCFullYear(), month: d.getUTCMonth() };
  }
  const toToday = () => (shown = { year: Number(today.slice(0, 4)), month: Number(today.slice(5, 7)) - 1 });
  onMount(() => {
    void getWeekStart().then((w) => (monday = w === 'monday'));
    const on: [string, () => void][] = [['calendar.prev', () => step(-1)], ['calendar.next', () => step(1)], ['calendar.today', toToday]];
    for (const [n, f] of on) addEventListener(`command:${n}`, f);
    return () => on.forEach(([n, f]) => removeEventListener(`command:${n}`, f));
  });
  const fire = (id: string) => dispatchEvent(new CustomEvent(`command:${id}`));
</script>

<div class="cal">
  <div class="bar">
    <IconButton icon="chevron-left" label="Previous month" shortcut={shortcutFor('calendar.prev')} onclick={() => fire('calendar.prev')} />
    <IconButton icon="chevron-right" label="Next month" shortcut={shortcutFor('calendar.next')} onclick={() => fire('calendar.next')} />
    <h2>{title}</h2>
    <Button onclick={() => fire('calendar.today')}>Today</Button>
  </div>
  {#if data.loaded && data.rows.length === 0}
    <EmptyState title="No projects in this view" hint="Projects that match this view will show up here." />
  {:else}
    <div class="grid" role="grid" aria-label={title}>
      {#each weekdays as w (w)}<span class="wd" role="columnheader">{w}</span>{/each}
      {#each days as d (d.day)}
        <div class="day" role="gridcell" data-day={d.day} data-today={d.day === today ? '' : undefined} data-out={d.inMonth ? undefined : ''}>
          <span class="n">{Number(d.day.slice(8))}</span>
          {#each chips.filter((c) => c.day === d.day) as c (`${c.row.project.id}-${c.kind}`)}
            <button type="button" class="chip" data-kind={c.kind} data-slug={c.row.project.slug} onclick={() => navigate(`/project/${c.row.project.slug}`)}>
              <span class="k">{c.kind === 'start' ? 'Start' : 'Target'}</span>{c.row.project.name}
            </button>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .cal { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .bar { flex: none; display: flex; align-items: center; gap: var(--s-3); height: var(--row-h); padding: 0 var(--s-6); }
  h2 { margin: 0 auto 0 var(--s-4); font-size: var(--fs-lg); font-weight: 600; }
  .grid { flex: 1; min-height: 0; overflow: auto; display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); grid-template-rows: auto; grid-auto-rows: minmax(var(--s-10), auto); border-top: 1px solid var(--border); }
  .wd { padding: var(--s-3) var(--s-4); color: var(--muted); font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; border-bottom: 1px solid var(--border); grid-auto-rows: auto; }
  .day { display: flex; flex-direction: column; gap: var(--s-1); padding: var(--s-2) var(--s-3); border-right: 1px solid var(--border); border-bottom: 1px solid var(--border); min-width: 0; }
  .day[data-out] { background: var(--raised); color: var(--muted); }
  .n { font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
  .day[data-today] .n { display: inline-grid; place-items: center; align-self: flex-start; min-width: var(--s-7); border-radius: var(--r-round); background: var(--signal); color: var(--signal-ink); font-weight: 700; }
  .chip { display: flex; align-items: center; gap: var(--s-2); min-width: 0; padding: 0 var(--s-3); border: 0; border-radius: var(--r-sm); background: var(--control); color: var(--ink); font: inherit; font-size: var(--fs-xs); text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; }
  .chip[data-kind='target'] { background: var(--accent); color: var(--accent-ink); }
  .chip:hover { filter: brightness(0.94); }
  .k { font-weight: 700; opacity: 0.75; }
</style>

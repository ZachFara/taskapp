<script lang="ts">
  import RowDialogHost from './RowDialogHost.svelte';
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { SyncStatus } from '../lib/bindings';
  import { shortcutFor } from '../lib/commands';
  import { refreshAll } from '../lib/stores.svelte';
    import Contour from '../ui/Contour.svelte';
  import Icon from '../ui/Icon.svelte';
  import CommandButton from './CommandButton.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Tooltip from '../ui/Tooltip.svelte';
  import Splitter from './Splitter.svelte';
  import { clampWidth, readFlag, readWidth, writeFlag, writeWidth } from '../lib/paneSize';
  import '../lib/interfaceSize.svelte'; // applies the saved interface size before the first paint
  import { ms, replay } from '../lib/motion.svelte'; // also applies the saved animation speed before the first paint

  // The frame: sidebar and top bar are one field, the page is a sheet set into it, the issue panel docks inside the sheet.
  let {
    sidebar,
    panel,
    children,
    crumb = [],
    panelOpen = false,
    onsearch,
    route = '',
    focus = false,
  }: { sidebar?: Snippet; panel?: Snippet; children: Snippet; crumb?: { label: string; href?: string }[]; panelOpen?: boolean; onsearch?: () => void; route?: string; focus?: boolean } = $props();

  let sync = $state<SyncStatus | null>(null);
  const syncError = $derived(
    sync ? Object.values(sync).map((s) => (s === 'Off' ? undefined : s?.Error?.message)).find((m) => m !== undefined) : undefined,
  );

  // Sidebar and panel widths are per device; the commands below are the keys and the menu entries for them.
  const SIDE = { key: 'cairn.sidebarWidth', def: 232, min: 180, max: 360 };
  const PANEL = { key: 'cairn.panelWidth', def: 380, min: 280 };
  const COLLAPSED = 'cairn.sidebarCollapsed';
  let sideW = $state<number | null>(readWidth(SIDE.key));
  let panelW = $state<number | null>(readWidth(PANEL.key));
  let collapsed = $state(readFlag(COLLAPSED));
  let side: HTMLElement;
  // Widths are CSS px. Pointer positions are visual px, so they are divided by the interface zoom.
  const zoom = () => Number(getComputedStyle(document.documentElement).getPropertyValue('--ui-zoom')) || 1;
  const sideWidth = () => sideW ?? (parseFloat(getComputedStyle(side).getPropertyValue('--sidebar-w')) || SIDE.def);
  let sheet: HTMLElement;
  let panelEl: HTMLElement;
  // A page change replays a quick rise on the sheet's content (the issue page slides in from the right instead).
  let mainEl: HTMLElement;
  let shown = route;
  $effect(() => {
    if (route === shown) return;
    shown = route;
    mainEl.style.setProperty('--cx-px', route.startsWith('/issue/') ? '24px' : '0px');
    replay(mainEl, 'cx-page');
  });
  // The panel may take 70% of the sheet at most, so the list keeps room.
  const clampSide = (w: number) => clampWidth(w, SIDE.min, SIDE.max);
  const clampPanel = (w: number) => clampWidth(w, PANEL.min, sheet.clientWidth * 0.7);
  const setSide = (w: number | null) => {
    sideW = w === null ? null : clampSide(w);
    writeWidth(SIDE.key, sideW);
  };
  const setPanel = (w: number | null) => {
    panelW = w === null ? null : clampPanel(w);
    writeWidth(PANEL.key, panelW);
  };
  const panelWidth = () => panelW ?? (parseFloat(getComputedStyle(panelEl).getPropertyValue('--panel-w')) || PANEL.def);

  onMount(() => {
    void refreshAll().catch(() => {});
    void api.sync.status().then((s) => (sync = s)).catch(() => {});
    const on = (type: string, fn: () => void) => {
      addEventListener(type, fn);
      return () => removeEventListener(type, fn);
    };
    const offs = [
      on('command:sidebar.toggle', () => {
        collapsed = !collapsed;
        writeFlag(COLLAPSED, collapsed);
      }),
      on('command:pane.sidebar.wider', () => setSide(sideWidth() + 16)),
      on('command:pane.sidebar.narrower', () => setSide(sideWidth() - 16)),
      on('command:pane.sidebar.reset', () => setSide(null)),
      on('command:pane.panel.wider', () => setPanel(panelWidth() + 16)),
      on('command:pane.panel.narrower', () => setPanel(panelWidth() - 16)),
      on('command:pane.panel.reset', () => setPanel(null)),
    ];
    return () => offs.forEach((f) => f());
  });

  // The legend is read from the registry, so a rebound key or a new command shows up without touching this list.
  const legend: [string[], string][] = [
    [['move.down', 'move.up'], 'move'],
    [['select.toggle'], 'select'],
    [['move.open'], 'open'],
    [['issue.status'], 'status'],
    [['palette.open'], 'commands'],
  ];
  const hints = $derived(
    legend
      .map(([ids, text]) => ({ text, keys: ids.map((id) => shortcutFor(id)).filter((k): k is string => !!k) }))
      .filter((h) => h.keys.length),
  );

</script>

<div class="app cairn-root" data-on-frame style:--sidebar-w={collapsed || focus ? '0px' : sideW === null ? null : `${sideW}px`}>
  <div class="brand">
    <span class="mark"><Icon name="cairn" size={24} /></span>
    <span class="wordmark">Cairn</span>
  </div>
  <header class="topbar">
    <nav class="crumb" aria-label="Breadcrumb">
      {#each crumb as part, i (i)}
        {#if i}<Icon name="chevron-right" size={12} />{/if}
        {#if i === crumb.length - 1}<b aria-current="page">{part.label}</b>
        {:else if part.href}<a href={part.href} onclick={(e) => { e.preventDefault(); navigate(part.href!); }}>{part.label}</a>
        {:else}<span>{part.label}</span>{/if}
      {/each}
    </nav>
    <Tooltip label="Search or run a command" shortcut={shortcutFor('palette.open')}>
      {#snippet children(props)}
        <button {...props} class="search" type="button" aria-label="Search or run a command" onclick={() => onsearch?.()}>
          <Icon name="search" />
          <span>Search or run a command</span>
          <span data-kbd-inherit><Kbd keys={shortcutFor('palette.open') ?? 'mod+k'} /></span>
        </button>
      {/snippet}
    </Tooltip>
    <div class="tools">
      <CommandButton command="help.shortcuts" iconOnly />
      <CommandButton command="theme.cycle" iconOnly />
    </div>
  </header>
  <aside class="sidebar" bind:this={side} hidden={collapsed || focus}>
    {@render sidebar?.()}
    <div class="edge side"><Splitter label="Resize sidebar" clamp={clampSide} fromPointer={(x) => x / zoom()} onchange={setSide} onreset={() => setSide(null)} /></div>
  </aside>
  <div class="sheet" bind:this={sheet} data-panel={panelOpen ? '' : undefined}>
    <div class="main">
      <main bind:this={mainEl}>{@render children()}</main>
      <RowDialogHost />
      <footer class="legend">
        {#each hints as h (h.text)}
          <span><Kbd keys={h.keys} />{h.text}</span>
        {/each}
        <span class="sync">
          {#if syncError !== undefined}
            <span class="dot error" role="status" aria-label={syncError}></span> {syncError}
          {:else}
            <span class="dot"></span> Sync ok
          {/if}
        </span>
      </footer>
    </div>
    {#if panelOpen}
      <aside class="panel" in:fly={{ x: 40, duration: ms('--dur-3'), easing: cubicOut }} out:fly={{ x: 40, duration: ms('--dur-out') }} bind:this={panelEl} style:--panel-w={panelW === null ? null : `${panelW}px`}>
        <div class="edge panel-edge">
          <Splitter label="Resize panel" clamp={clampPanel} fromPointer={(x) => (sheet.getBoundingClientRect().right - x) / zoom()} onchange={setPanel} onreset={() => setPanel(null)} />
        </div>
        <div class="contour" aria-hidden="true"><Contour rings={11} size={380} /></div>
        <div class="body">{@render panel?.()}</div>
      </aside>
    {/if}
  </div>
</div>

<style>
  .app {
    display: grid;
    zoom: var(--ui-zoom, 1);
    /* Fill the window by its edges, not by a height: Chromium multiplies a height by the zoom and WebKit (the Mac
       app) does not, so any height here is wrong in one of them. */
    position: fixed;
    inset: 0;
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
    grid-template-rows: var(--topbar-h) minmax(0, 1fr);
    grid-template-areas: 'brand top' 'side sheet';
    background: var(--frame);
    color: var(--frame-ink);
    font-size: var(--fs-md);
  }
  .brand { grid-area: brand; display: flex; align-items: center; gap: var(--s-5); padding: 0 var(--s-6); }
  .mark { display: flex; color: var(--frame-ink); }
  .wordmark { font-size: var(--fs-wordmark); font-weight: 700; letter-spacing: -0.02em; font-variation-settings: 'wdth' 90; }
  .topbar { grid-area: top; display: grid; grid-template-columns: 1fr minmax(240px, 420px) 1fr; align-items: center; gap: var(--s-4); padding: 0 var(--s-4) 0 var(--s-2); color: var(--frame-muted); }
  .crumb { display: flex; align-items: center; gap: var(--s-3); white-space: nowrap; font-weight: 500; min-width: 0; }
  .crumb > :not(:global(svg)) { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .crumb b { color: var(--frame-ink); font-weight: 600; }
  .crumb a { color: inherit; text-decoration: none; }
  .crumb a:hover { color: var(--frame-ink); text-decoration: underline; }
  .search { display: flex; align-items: center; gap: var(--s-4); height: var(--ctl-h); padding: 0 var(--s-3) 0 var(--s-5); border: 0; border-radius: var(--r-ctl); background: var(--frame-inset); color: var(--frame-muted); font: inherit; text-align: left; }
  .search:hover { color: var(--frame-ink); }
  .search span:first-of-type { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tools { display: flex; align-items: center; justify-content: flex-end; gap: var(--s-2); }
  .tools :global(.ib) { color: var(--frame-ink); background: var(--frame-raised); }
  .tools :global(.ib:hover:not(:disabled)) { background: var(--frame-inset); color: var(--frame-ink); }
  .sidebar { position: relative; grid-area: side; min-height: 0; overflow: auto; }
  .sheet { grid-area: sheet; display: flex; min-width: 0; min-height: 0; margin: 0 var(--sheet-inset) var(--sheet-inset) 0; overflow: hidden; background: var(--sheet); color: var(--ink); border: 1px solid var(--border); border-radius: var(--r-sheet); position: relative; }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  main { flex: 1; min-height: 0; overflow: auto; }
  .legend { flex: none; display: flex; align-items: center; gap: var(--s-7); height: var(--row-h); padding: 0 var(--s-7); background: var(--raised); color: var(--muted); font-size: var(--fs-sm); }
  .legend > span { display: inline-flex; align-items: center; gap: var(--s-3); white-space: nowrap; }
  .sync { margin-left: auto; }
  .dot { width: var(--s-4); height: var(--s-4); border-radius: var(--r-round); background: var(--positive); }
  .dot.error { background: var(--signal); }
  .panel { position: relative; flex: none; width: var(--panel-w); min-height: 0; display: flex; flex-direction: column; background: var(--raised); border-left: 1px solid var(--border); overflow: hidden; }
  /* The panel moves by transform (fly), not width: the list beside it keeps its layout, and dims a little. */
  .main { transition: opacity var(--dur-3) var(--ease); }
  .sheet[data-panel] .main { opacity: 0.85; }
  /* Narrow window: the panel floats over the sheet so the list keeps its width. */
  @media (max-width: 1100px) {
    .panel { position: absolute; top: 0; right: 0; bottom: 0; z-index: var(--z-sticky); box-shadow: var(--shadow-float); }
  }
  .body { position: relative; flex: 1; min-height: 0; overflow: auto; }
  .contour { position: absolute; right: calc(var(--s-10) * -2); bottom: calc(var(--s-10) * -1); color: var(--border); pointer-events: none; }
  .edge { position: absolute; top: 0; bottom: 0; width: var(--s-4); z-index: var(--z-sticky); }
  .edge.side { right: 0; }
  .edge.panel-edge { left: calc(var(--s-2) * -1); }
  .brand { overflow: hidden; }
</style>

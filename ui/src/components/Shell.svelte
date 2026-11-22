<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { SyncStatus } from '../lib/bindings';
  import { shortcutFor } from '../lib/commands';
  import { refreshAll } from '../lib/stores.svelte';
    import Contour from '../ui/Contour.svelte';
  import Icon from '../ui/Icon.svelte';
  import CommandButton from './CommandButton.svelte';
  import Kbd from '../ui/Kbd.svelte';

  // The frame: sidebar and top bar are one field, the page is a sheet set into it, the issue panel docks inside the sheet.
  let {
    sidebar,
    panel,
    children,
    crumb = [],
    panelOpen = false,
    onsearch,
  }: { sidebar?: Snippet; panel?: Snippet; children: Snippet; crumb?: string[]; panelOpen?: boolean; onsearch?: () => void } = $props();

  let sync = $state<SyncStatus | null>(null);
  const syncError = $derived(
    sync ? Object.values(sync).map((s) => (s === 'Off' ? undefined : s?.Error?.message)).find((m) => m !== undefined) : undefined,
  );

  onMount(() => {
    void refreshAll().catch(() => {});
    void api.sync.status().then((s) => (sync = s)).catch(() => {});
  });

  // The legend is read from the registry, so a rebound key or a new command shows up without touching this list.
  const legend: [string[], string][] = [
    [['move.down', 'move.up'], 'move'],
    [['move.select'], 'select'],
    [['move.open'], 'open'],
    [['issue.status'], 'status'],
    [['palette.open'], 'commands'],
  ];
  const keyName = (s: string) => (s.startsWith('mod+') ? `⌘${s.slice(4).toUpperCase()}` : s === 'enter' ? '↵' : s === 'escape' ? 'Esc' : s);
  const hints = $derived(
    legend
      .map(([ids, text]) => ({ text, keys: ids.map((id) => shortcutFor(id)).filter((k): k is string => !!k).map(keyName) }))
      .filter((h) => h.keys.length),
  );

  // The panel is dragged by its left edge; its width lives in the --panel-w token's place, clamped to the sheet.
  let width = $state<number | null>(null);
  let sheet: HTMLElement;
  function resize(e: PointerEvent) {
    const move = (m: PointerEvent) => {
      const room = sheet.getBoundingClientRect().right - m.clientX;
      width = Math.round(Math.min(sheet.clientWidth * 0.7, Math.max(280, room)));
    };
    const stop = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', stop);
    };
    e.preventDefault();
    addEventListener('pointermove', move);
    addEventListener('pointerup', stop);
  }
</script>

<div class="app" data-on-frame>
  <div class="brand">
    <span class="mark"><Icon name="cairn" size={24} /></span>
    <span class="wordmark">Cairn</span>
  </div>
  <header class="topbar">
    <nav class="crumb" aria-label="Breadcrumb">
      {#each crumb as part, i (i)}
        {#if i}<Icon name="chevron-right" size={12} />{/if}
        {#if i === crumb.length - 1}<b aria-current="page">{part}</b>{:else}<span>{part}</span>{/if}
      {/each}
    </nav>
    <button class="search" type="button" aria-label="Search or run a command" onclick={() => onsearch?.()}>
      <Icon name="search" />
      <span>Search or run a command</span>
      <span data-kbd-inherit><Kbd keys="⌘K" /></span>
    </button>
    <div class="tools">
      <CommandButton command="issue.create" variant="primary" />
      <CommandButton command="help.shortcuts" iconOnly />
      <CommandButton command="theme.cycle" iconOnly />
    </div>
  </header>
  <aside class="sidebar">{@render sidebar?.()}</aside>
  <div class="sheet" bind:this={sheet}>
    <div class="main">
      <main>{@render children()}</main>
      <footer class="legend">
        {#each hints as h (h.text)}
          <span><Kbd keys={h.keys} />{h.text}</span>
        {/each}
        <span class="sync">
          {#if syncError !== undefined}
            <span class="dot error" role="status" title={syncError}></span> {syncError}
          {:else}
            <span class="dot"></span> Sync ok
          {/if}
        </span>
      </footer>
    </div>
    {#if panelOpen}
      <aside class="panel" style:--panel-w={width === null ? null : `${width}px`}>
        <button class="grip" type="button" aria-label="Resize panel" onpointerdown={resize}></button>
        <div class="contour" aria-hidden="true"><Contour rings={11} size={380} /></div>
        <div class="body">{@render panel?.()}</div>
      </aside>
    {/if}
  </div>
</div>

<style>
  .app {
    display: grid;
    height: 100vh;
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
  .crumb b { color: var(--frame-ink); font-weight: 600; }
  .search { display: flex; align-items: center; gap: var(--s-4); height: var(--ctl-h); padding: 0 var(--s-3) 0 var(--s-5); border: 0; border-radius: var(--r-ctl); background: var(--frame-inset); color: var(--frame-muted); font: inherit; text-align: left; }
  .search:hover { color: var(--frame-ink); }
  .search span:first-of-type { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tools { display: flex; align-items: center; justify-content: flex-end; gap: var(--s-2); }
  .tools :global(.ib) { color: var(--frame-ink); background: var(--frame-raised); }
  .tools :global(.ib:hover:not(:disabled)) { background: var(--frame-inset); color: var(--frame-ink); }
  .sidebar { grid-area: side; min-height: 0; overflow: auto; }
  .sheet { grid-area: sheet; display: flex; min-width: 0; min-height: 0; margin: 0 var(--sheet-inset) var(--sheet-inset) 0; overflow: hidden; background: var(--sheet); color: var(--ink); border: 1px solid var(--border); border-radius: var(--r-sheet); position: relative; }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  main { flex: 1; min-height: 0; overflow: auto; }
  .legend { flex: none; display: flex; align-items: center; gap: var(--s-7); height: var(--row-h); padding: 0 var(--s-7); background: var(--raised); color: var(--muted); font-size: var(--fs-sm); }
  .legend > span { display: inline-flex; align-items: center; gap: var(--s-3); white-space: nowrap; }
  .sync { margin-left: auto; }
  .dot { width: var(--s-4); height: var(--s-4); border-radius: var(--r-round); background: var(--positive); }
  .dot.error { background: var(--signal); }
  .panel { position: relative; flex: none; width: var(--panel-w); min-height: 0; display: flex; flex-direction: column; background: var(--raised); border-left: 1px solid var(--border); overflow: hidden; }
  .body { position: relative; flex: 1; min-height: 0; overflow: auto; }
  .contour { position: absolute; right: calc(var(--s-10) * -2); bottom: calc(var(--s-10) * -1); color: var(--border); pointer-events: none; }
  .grip { position: absolute; z-index: var(--z-sticky); left: calc(var(--s-2) * -1); top: 0; bottom: 0; width: var(--s-4); padding: 0; border: 0; background: transparent; cursor: col-resize; }
  .grip:hover, .grip:active { background: var(--control); opacity: 0.5; }
</style>

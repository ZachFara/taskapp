<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { SyncStatus } from '../lib/bindings';
  import { defaultKeymap, registerShortcut } from '../lib/keymap';
  import { navigate } from '../app/routes';
  import { refreshAll, views } from '../lib/stores.svelte';
  import CommandPalette from './CommandPalette.svelte';
  import QuickAdd from './QuickAdd.svelte';

  // The sidebar and issue panel are slots filled by other screens; they stay empty until those exist.
  let { sidebar, panel, children }: { sidebar?: Snippet; panel?: Snippet; children: Snippet } = $props();

  let overlay = $state<'palette' | 'quickadd' | null>(null);
  let sync = $state<SyncStatus | null>(null);
  const syncError = $derived(
    sync ? Object.values(sync).map((s) => (s === 'Off' ? undefined : s?.Error?.message)).find((m) => m !== undefined) : undefined,
  );

  onMount(() => {
    void refreshAll().catch(() => {});
    void api.sync.status().then((s) => (sync = s)).catch(() => {});
    const off = [
      registerShortcut(defaultKeymap.palette, 'palette', () => (overlay = 'palette')),
      registerShortcut(defaultKeymap.create, 'create', () => (overlay = 'quickadd')),
      registerShortcut(defaultKeymap.search, 'search', () => (overlay = 'palette')),
      // No inbox screen exists: g i opens the first (default) view; g v opens the palette, which lists every view.
      registerShortcut(defaultKeymap.inbox, 'inbox', () => views[0] && navigate(`/view/${views[0].id}`)),
      registerShortcut(defaultKeymap.views, 'views', () => (overlay = 'palette')),
    ];
    return () => off.forEach((f) => f());
  });
</script>

<div class="shell">
  <aside class="sidebar">{@render sidebar?.()}</aside>
  <main>{@render children()}</main>
  <aside class="panel">{@render panel?.()}</aside>
  <div class="status">
    {#if syncError !== undefined}
      <span class="dot error" role="status" title={syncError}></span> {syncError}
    {:else}
      <span class="dot"></span> Sync ok
    {/if}
  </div>
</div>

{#if overlay === 'palette'}
  <CommandPalette onclose={() => (overlay = null)} oncreate={() => (overlay = 'quickadd')} />
{:else if overlay === 'quickadd'}
  <QuickAdd onclose={() => (overlay = null)} />
{/if}

<style>
  .shell { display: grid; grid-template-columns: 240px 1fr auto; grid-template-rows: 1fr 24px; height: 100vh; font: 13px system-ui, sans-serif; color: #222; }
  .sidebar { background: #f6f6f8; border-right: 1px solid #e4e4e8; overflow: auto; }
  main { overflow: auto; }
  .panel:not(:empty) { width: 360px; border-left: 1px solid #e4e4e8; overflow: auto; }
  .status { grid-column: 1 / -1; display: flex; align-items: center; gap: 6px; padding: 0 12px; border-top: 1px solid #e4e4e8; font-size: 11px; color: #666; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #4caf50; }
  .dot.error { background: #d32f2f; }
</style>

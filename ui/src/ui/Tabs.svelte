<script lang="ts">
  import { Tabs } from 'bits-ui';
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  import Tooltip from './Tooltip.svelte';

  // The layout switcher (List, Board, Table, ...). Arrow keys move between tabs.
  let {
    value = $bindable(''),
    tabs,
    label = 'Layout',
    segmented = false,
  }: { segmented?: boolean; value?: string; tabs: { id: string; label: string; icon?: string; shortcut?: string }[]; label?: string } = $props();

  // A tab's tooltip only repeats its visible name unless it has a shortcut or the text is hidden (1200px, below).
  let width = $state(Infinity);

  // The underline is one bar that slides to the active tab; it is measured after each change, and placed without
  // sliding the first time. Segmented controls fill the chosen segment instead.
  let list = $state<HTMLElement | null>(null);
  let ink = $state<{ x: number; w: number } | null>(null);
  let ready = $state(false);
  $effect(() => {
    void value;
    void tick().then(() => {
      const a = list?.querySelector<HTMLElement>('[data-state="active"]');
      if (!a) return;
      ink = { x: a.offsetLeft, w: a.offsetWidth };
      requestAnimationFrame(() => (ready = true));
    });
  });
</script>

<svelte:window bind:innerWidth={width} />

<Tabs.Root bind:value>
  <Tabs.List bind:ref={list} class={segmented ? 'cx-tabs cx-seg' : 'cx-tabs'} aria-label={label}>
    {#each tabs as t (t.id)}
      <Tooltip label={t.label} shortcut={t.shortcut} disabled={!t.shortcut && (segmented || !t.icon || width > 1200)}>
        {#snippet children(tp)}
          <!-- The wrapper takes the tooltip's attributes: spread on the tab they would overwrite its data-state. -->
          <span {...tp} class="cx-tabwrap">
            <Tabs.Trigger class="cx-tab" value={t.id} aria-label={t.label}>
              {#if t.icon}<Icon name={t.icon} />{/if}<span class="cx-tab-label">{t.label}</span>
            </Tabs.Trigger>
          </span>
        {/snippet}
      </Tooltip>
    {/each}
    {#if ink && !segmented}<span class="cx-ink" class:ready style:transform="translateX({ink.x}px)" style:width="{ink.w}px" aria-hidden="true"></span>{/if}
  </Tabs.List>
</Tabs.Root>

<style>
  :global(.cx-tabs) { position: relative; display: flex; gap: var(--s-1); height: var(--topbar-h); }
  :global(.cx-tabwrap) { display: flex; }
  :global(.cx-tab) {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 0 var(--s-5);
    margin-top: var(--s-1);
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-weight: 500;
    transition: color var(--dur-1) var(--ease), border-color var(--dur-2) var(--ease);
  }
  /* A segmented control: one outlined group, the chosen segment filled. Names always show. */
  :global(.cx-seg) { height: auto; gap: 0; padding: var(--s-1); border: 1px solid var(--border); border-radius: var(--r-ctl); background: var(--raised); }
  :global(.cx-seg .cx-tab) { margin: 0; height: var(--ctl-h); border: 0; border-radius: var(--r-sm); }
  :global(.cx-seg .cx-tab[data-state='active']) { background: var(--sheet); box-shadow: 0 0 0 1px var(--border); }
  :global(.cx-seg .cx-tab-label) { display: inline !important; }
  :global(.cx-tab:hover) { color: var(--ink); }
  :global(.cx-tab[data-state='active']) { color: var(--ink); font-weight: 600; }
  :global(.cx-ink) { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--ink); pointer-events: none; }
  :global(.cx-ink.ready) { transition: transform var(--dur-3) var(--ease), width var(--dur-3) var(--ease); }
  /* Tabs with an icon drop their text on narrow windows; the title and aria-label keep the name. */
  @media (max-width: 1200px) {
    :global(.cx-tab-label:not(:only-child)) { display: none; }
  }
</style>

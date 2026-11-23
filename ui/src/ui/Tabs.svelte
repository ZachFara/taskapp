<script lang="ts">
  import { Tabs } from 'bits-ui';
  import Icon from './Icon.svelte';
  import Tooltip from './Tooltip.svelte';

  // The layout switcher (List, Board, Table, ...). Arrow keys move between tabs.
  let {
    value = $bindable(''),
    tabs,
    label = 'Layout',
    segmented = false,
  }: { segmented?: boolean; value?: string; tabs: { id: string; label: string; icon?: string; shortcut?: string }[]; label?: string } = $props();
</script>

<Tabs.Root bind:value>
  <Tabs.List class={segmented ? 'cx-tabs cx-seg' : 'cx-tabs'} aria-label={label}>
    {#each tabs as t (t.id)}
      <Tooltip label={t.label} shortcut={t.shortcut}>
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
  </Tabs.List>
</Tabs.Root>

<style>
  :global(.cx-tabs) { display: flex; gap: var(--s-1); height: var(--topbar-h); }
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
    transition: color var(--dur-1) var(--ease);
  }
  /* A segmented control: one outlined group, the chosen segment filled. Names always show. */
  :global(.cx-seg) { height: auto; gap: 0; padding: var(--s-1); border: 1px solid var(--border); border-radius: var(--r-ctl); background: var(--raised); }
  :global(.cx-seg .cx-tab) { margin: 0; height: var(--ctl-h); border: 0; border-radius: var(--r-sm); }
  :global(.cx-seg .cx-tab[data-state='active']) { background: var(--sheet); box-shadow: 0 0 0 1px var(--border); }
  :global(.cx-seg .cx-tab-label) { display: inline !important; }
  :global(.cx-tab:hover) { color: var(--ink); }
  :global(.cx-tab[data-state='active']) { color: var(--ink); font-weight: 600; border-bottom-color: var(--ink); }
  /* Tabs with an icon drop their text on narrow windows; the title and aria-label keep the name. */
  @media (max-width: 1200px) {
    :global(.cx-tab-label:not(:only-child)) { display: none; }
  }
</style>

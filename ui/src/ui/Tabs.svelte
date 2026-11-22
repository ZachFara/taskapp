<script lang="ts">
  import { Tabs } from 'bits-ui';
  import Icon from './Icon.svelte';

  // The layout switcher (List, Board, Table, ...). Arrow keys move between tabs.
  let {
    value = $bindable(''),
    tabs,
    label = 'Layout',
  }: { value?: string; tabs: { id: string; label: string; icon?: string }[]; label?: string } = $props();
</script>

<Tabs.Root bind:value>
  <Tabs.List class="cx-tabs" aria-label={label}>
    {#each tabs as t (t.id)}
      <Tabs.Trigger class="cx-tab" value={t.id}>
        {#if t.icon}<Icon name={t.icon} />{/if}{t.label}
      </Tabs.Trigger>
    {/each}
  </Tabs.List>
</Tabs.Root>

<style>
  :global(.cx-tabs) { display: flex; gap: var(--s-1); height: var(--topbar-h); }
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
  :global(.cx-tab:hover) { color: var(--ink); }
  :global(.cx-tab[data-state='active']) { color: var(--ink); font-weight: 600; border-bottom-color: var(--ink); }
</style>

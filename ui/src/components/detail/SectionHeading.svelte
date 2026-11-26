<script lang="ts">
  import Icon from '../../ui/Icon.svelte';
  import IconButton from '../../ui/IconButton.svelte';

  // A detail section's heading: a small label, its count and one rule. Legend bands are for status groups only.
  let {
    name,
    count,
    collapsed = false,
    ontoggle,
    onadd,
  }: { name: string; count: number; collapsed?: boolean; ontoggle?: () => void; onadd?: () => void } = $props();
</script>

<div class="heading" data-section-heading={name}>
  <button type="button" class="toggle" aria-expanded={!collapsed} aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${name}`} onclick={ontoggle}>
    <span class="chev" data-collapsed={collapsed}><Icon name="chevron-down" size={12} /></span>
    <span class="name">{name}</span>
    <span class="count">{count}</span>
  </button>
  {#if onadd}<IconButton icon="plus" label={`Add to ${name}`} onclick={onadd} />{/if}
</div>

<style>
  .heading {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-5) 0 var(--s-2);
    border-bottom: 1px solid var(--border);
  }
  .toggle {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--ink);
    font-size: var(--fs-sm);
    font-weight: 600;
    text-align: start;
  }
  .chev { display: flex; transition: transform var(--dur-2) var(--ease); }
  .chev[data-collapsed='true'] { transform: rotate(-90deg); }
  .count { color: var(--muted); font-weight: 500; }
</style>

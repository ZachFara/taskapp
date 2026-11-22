<script lang="ts">
  import { Select } from 'bits-ui';
  import Icon from './Icon.svelte';

  let {
    value = $bindable(''),
    items,
    label,
    placeholder = 'Select',
    disabled = false,
  }: { value?: string; items: { value: string; label: string }[]; label: string; placeholder?: string; disabled?: boolean } = $props();
  const shown = $derived(items.find((i) => i.value === value)?.label);
</script>

<Select.Root type="single" bind:value {items} {disabled}>
  <Select.Trigger class="cx-select" aria-label={label}>
    <span class="v" data-empty={shown ? undefined : ''}>{shown ?? placeholder}</span>
    <Icon name="chevron-down" size={12} />
  </Select.Trigger>
  <Select.Portal>
    <Select.Content class="cx-surface cx-listbox" sideOffset={4}>
      <Select.Viewport>
        {#each items as item (item.value)}
          <Select.Item class="cx-item cx-option" value={item.value} label={item.label}>
            {#snippet children({ selected })}
              <span class="name">{item.label}</span>
              {#if selected}<Icon name="check" size={12} />{/if}
            {/snippet}
          </Select.Item>
        {/each}
      </Select.Viewport>
    </Select.Content>
  </Select.Portal>
</Select.Root>

<style>
  :global(.cx-select) {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-4);
    min-width: 140px;
    height: var(--ctl-h);
    padding: 0 var(--s-5);
    border: 1px solid var(--control);
    border-radius: var(--r-ctl);
    background: var(--sheet);
    color: var(--ink);
    font-weight: 500;
  }
  :global(.cx-select:hover:not([data-disabled])) { border-color: var(--muted); }
  :global(.cx-select[data-disabled]) { opacity: 0.55; }
  .v[data-empty] { color: var(--muted); }
  :global(.cx-listbox) { min-width: var(--bits-select-anchor-width); padding: var(--s-2); }
  :global(.cx-option) { cursor: default; }
  :global(.cx-option) .name { flex: 1; }
</style>

<script lang="ts">
  import { Checkbox } from 'bits-ui';
  import Icon from './Icon.svelte';

  let {
    checked = $bindable(false),
    indeterminate = $bindable(false),
    label,
    disabled = false,
    onchange,
  }: { checked?: boolean; indeterminate?: boolean; label: string; disabled?: boolean; onchange?: (checked: boolean) => void } = $props();
</script>

<Checkbox.Root class="cx-check" bind:checked bind:indeterminate {disabled} onCheckedChange={onchange} aria-label={label}>
  {#snippet children({ checked, indeterminate })}
    {#if indeterminate}<Icon name="plus" size={12} />{:else if checked}<Icon name="check" size={12} />{/if}
  {/snippet}
</Checkbox.Root>

<style>
  :global(.cx-check) {
    width: 16px;
    height: 16px;
    display: inline-grid;
    place-items: center;
    padding: 0;
    flex: none;
    border: 1px solid var(--control);
    border-radius: var(--r-sm);
    background: var(--sheet);
    color: var(--accent-ink);
    transition: background var(--dur-1) var(--ease), border-color var(--dur-1) var(--ease);
  }
  :global(.cx-check:hover:not([data-disabled])) { border-color: var(--muted); }
  :global(.cx-check[data-state='checked']), :global(.cx-check[data-state='indeterminate']) {
    background: var(--accent);
    border-color: var(--accent);
    animation: cx-pop var(--dur-2) var(--ease-spring);
  }
  :global(.cx-check[data-disabled]) { opacity: 0.45; cursor: default; }
</style>

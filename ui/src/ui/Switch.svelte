<script lang="ts">
  import { Switch } from 'bits-ui';

  let { checked = $bindable(false), label, disabled = false }: { checked?: boolean; label: string; disabled?: boolean } = $props();
</script>

<Switch.Root class="cx-switch" bind:checked {disabled} aria-label={label}>
  <Switch.Thumb class="cx-thumb" />
</Switch.Root>

<style>
  :global(.cx-switch) {
    width: 30px;
    height: 18px;
    padding: 2px;
    flex: none;
    display: inline-flex;
    border: 0;
    border-radius: var(--r-round);
    background: var(--control);
    transition: background var(--dur-2) var(--ease);
  }
  :global(.cx-switch[data-state='checked']) { background: var(--accent); }
  :global(.cx-switch[data-disabled]) { opacity: 0.45; cursor: default; }
  :global(.cx-thumb) {
    width: 14px;
    height: 14px;
    border-radius: var(--r-round);
    background: var(--sheet);
    transition: transform var(--dur-2) var(--ease);
  }
  :global(.cx-switch[data-state='checked'] .cx-thumb) { transform: translateX(12px); background: var(--accent-ink); }
</style>

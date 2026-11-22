<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import Icon from './Icon.svelte';
  import Kbd from './Kbd.svelte';

  let {
    label,
    icon,
    shortcut,
    danger = false,
    disabled = false,
    onSelect,
  }: { label: string; icon?: string; shortcut?: string | string[]; danger?: boolean; disabled?: boolean; onSelect?: () => void } = $props();
</script>

<DropdownMenu.Item class="cx-item" {disabled} {onSelect} data-danger={danger || undefined}>
  <span class="lead">{#if icon}<Icon name={icon} />{/if}</span>
  <span class="name">{label}</span>
  {#if shortcut}<Kbd keys={shortcut} />{/if}
</DropdownMenu.Item>

<style>
  :global(.cx-item) {
    display: flex;
    align-items: center;
    gap: var(--s-5);
    height: var(--ctl-h);
    padding: 0 var(--s-4);
    border-radius: var(--r-sm);
    font-weight: 500;
    cursor: default;
    user-select: none;
  }
  :global(.cx-item[data-highlighted]) { background: var(--active); }
  :global(.cx-item[data-disabled]) { opacity: 0.45; }
  :global(.cx-item[data-danger]) { color: var(--signal); }
  .lead { width: var(--icon); display: grid; color: var(--muted); }
  :global(.cx-item[data-danger]) .lead { color: inherit; }
  .name { flex: 1; }
</style>

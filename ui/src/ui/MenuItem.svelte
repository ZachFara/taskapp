<script lang="ts">
  import { ContextMenu, DropdownMenu } from 'bits-ui';
  import { getContext } from 'svelte';
  import Icon from './Icon.svelte';
  import Kbd from './Kbd.svelte';

  let {
    label,
    icon,
    shortcut,
    danger = false,
    disabled = false,
    indent = 0,
    onSelect,
  }: { label: string; icon?: string; shortcut?: string | string[]; danger?: boolean; disabled?: boolean; indent?: number; onSelect?: () => void } = $props();
  // ContextMenu sets this so the same MenuItem works under both menus.
  const Item = getContext('cx-menu') === 'context' ? ContextMenu.Item : DropdownMenu.Item;
</script>

<Item class="cx-item" {disabled} {onSelect} data-danger={danger || undefined} style={indent ? `padding-left: calc(var(--s-4) + ${indent} * var(--s-5))` : undefined}>
  <span class="lead">{#if icon}<Icon name={icon} />{/if}</span>
  <span class="name">{label}</span>
  {#if shortcut}<Kbd keys={shortcut} />{/if}
</Item>

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

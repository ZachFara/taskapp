<script lang="ts">
  import Icon from './Icon.svelte';
  import Tooltip from './Tooltip.svelte';

  // `label` is required: an icon alone is not an accessible name, and it feeds the tooltip.
  let {
    icon,
    label,
    shortcut,
    pressed,
    disabled = false,
    onclick,
  }: {
    icon: string;
    label: string;
    shortcut?: string | string[];
    pressed?: boolean;
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
  } = $props();
</script>

<Tooltip {label} {shortcut}>
  {#snippet children(props)}
    <button {...props} class="ib" type="button" aria-label={label} aria-pressed={pressed} {disabled} {onclick}>
      <Icon name={icon} />
    </button>
  {/snippet}
</Tooltip>

<style>
  .ib {
    width: var(--ctl-h);
    height: var(--ctl-h);
    display: inline-grid;
    place-items: center;
    padding: 0;
    border: 0;
    border-radius: var(--r-ctl);
    background: transparent;
    color: var(--muted);
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  .ib:hover:not(:disabled), .ib[aria-pressed='true'] { background: var(--active); color: var(--ink); }
  .ib:disabled { opacity: 0.45; cursor: default; }
</style>

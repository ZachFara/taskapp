<script lang="ts">
  import type { StatusCategory } from '../lib/bindings';
  import EntityIcon from './EntityIcon.svelte';

  // Linear-style circles: backlog dashed, unstarted empty, started half, completed filled with a check, canceled with a cross.
  // A status with its own icon shows that icon in its colour instead; without one it uses its colour, else the stone token.
  let {
    category,
    icon = null,
    color = null,
    size = 16,
    label,
  }: { category: StatusCategory; icon?: string | null; color?: string | null; size?: number; label?: string } = $props();
</script>

{#if icon}
  <EntityIcon {icon} {color} {size} />
{:else}
  <svg
    class="glyph"
    style:color={color ? `var(--c-${color}, var(--stone))` : undefined}
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    role="img"
    aria-label={label ?? category}
    data-category={category}
  >
    {#if category === 'Backlog'}
      <circle cx="8" cy="8" r="6" stroke-dasharray="2.2 2" />
    {:else if category === 'Unstarted'}
      <circle cx="8" cy="8" r="6" />
    {:else if category === 'Started'}
      <circle cx="8" cy="8" r="6" />
      <path d="M8 2a6 6 0 0 1 0 12z" fill="currentColor" />
    {:else if category === 'Completed'}
      <circle cx="8" cy="8" r="6.4" fill="currentColor" />
      <path d="M5.2 8.2l1.9 1.9 3.7-3.9" stroke="var(--sheet)" stroke-width="1.6" />
    {:else}
      <circle cx="8" cy="8" r="6" />
      <path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4" />
    {/if}
  </svg>
{/if}

<style>
  .glyph { flex: none; display: block; color: var(--stone); }
</style>

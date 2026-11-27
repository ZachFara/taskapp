<script lang="ts">
  import { Select } from 'bits-ui';
  import type { StatusCategory } from '../lib/bindings';
  import EntityIcon from './EntityIcon.svelte';
  import Icon from './Icon.svelte';
  import StatusGlyph from './StatusGlyph.svelte';

  // `glyph` draws a leading mark: a status glyph when it has a category, else an icon (a null icon is the plain colour circle).
  type Glyph = { category?: StatusCategory; icon?: string | null; color?: string | null };

  let {
    value = $bindable(''),
    items,
    label,
    placeholder = 'Select',
    disabled = false,
  }: { value?: string; items: { value: string; label: string; glyph?: Glyph }[]; label: string; placeholder?: string; disabled?: boolean } = $props();
  const current = $derived(items.find((i) => i.value === value));
  const shown = $derived(current?.label);
</script>

{#snippet mark(g: Glyph)}
  {#if g.category}<StatusGlyph category={g.category} icon={g.icon} color={g.color} size={14} />{:else}<EntityIcon icon={g.icon} color={g.color} size={14} />{/if}
{/snippet}

<Select.Root type="single" bind:value {items} {disabled}>
  <Select.Trigger class="cx-select" aria-label={label}>
    {#if current?.glyph}{@render mark(current.glyph)}{/if}
    <span class="v" data-empty={shown ? undefined : ''}>{shown ?? placeholder}</span>
    <Icon name="chevron-down" size={12} />
  </Select.Trigger>
  <Select.Portal>
    <Select.Content class="cx-surface cx-listbox" sideOffset={4}>
      <Select.Viewport>
        {#each items as item (item.value)}
          <Select.Item class="cx-item cx-option" value={item.value} label={item.label}>
            {#snippet children({ selected })}
              {#if item.glyph}{@render mark(item.glyph)}{/if}
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
  .v { flex: 1; text-align: left; }
  .v[data-empty] { color: var(--muted); }
  :global(.cx-listbox) { min-width: calc(var(--bits-select-anchor-width) / var(--ui-zoom, 1)); padding: var(--s-2); }
  :global(.cx-option) { cursor: default; }
  :global(.cx-option) .name { flex: 1; }
</style>

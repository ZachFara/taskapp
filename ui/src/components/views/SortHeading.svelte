<script lang="ts">
  import type { FieldRef, Sort } from '../../lib/bindings';
  import { sameField } from '../../lib/viewSort';

  let { field, label, display = label, sort, onclick }: { field: FieldRef; label: string; display?: string; sort: Sort | null; onclick: (field: FieldRef) => void } = $props();
  const active = $derived(sort && sameField(sort.field, field));
</script>

<button type="button" aria-label={`Sort by ${label}`} title={`Sort by ${label}`} onclick={() => onclick(field)}>
  <span>{display}</span>{#if active}<span aria-hidden="true">{sort?.descending ? '↓' : '↑'}</span>{/if}
</button>

<style>
  button { display: inline-flex; align-items: center; gap: var(--s-2); max-width: 100%; padding: 0; border: 0; background: transparent; color: inherit; font: inherit; letter-spacing: inherit; text-transform: inherit; cursor: pointer; }
  button:hover { color: var(--ink); }
  button:focus-visible { outline: 2px solid var(--signal); outline-offset: 2px; }
  span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>

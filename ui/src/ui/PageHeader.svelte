<script lang="ts">
  import { setContext, type Snippet } from 'svelte';
  import EntityIcon from './EntityIcon.svelte';
  import TextInput from './TextInput.svelte';

  let {
    title,
    editable = false,
    onrename,
    icon,
    color,
    tabs,
    filters,
    actions,
    iconEdit,
  }: { title: string | Snippet; editable?: boolean; onrename?: (value: string) => void; icon?: string; color?: string; tabs?: Snippet; filters?: Snippet; actions?: Snippet; iconEdit?: Snippet } = $props();
  // When the row runs out of room (window width, interface size, a long title), the tabs drop their names first;
  // only then does the title shorten. `need` is the width the row wanted with names shown.
  let top: HTMLElement | undefined = $state();
  let heading: HTMLElement | undefined = $state();
  let strip: HTMLElement | undefined = $state();
  let compact = $state(false);
  let need = 0;
  setContext('pageHeaderCompact', () => compact);
  function fit() {
    if (!top || !heading) return;
    const spill = (e: HTMLElement | undefined) => (e ? e.scrollWidth - e.clientWidth : 0);
    const over = spill(top) + spill(heading) + spill(strip);
    if (!compact && over > 0) [need, compact] = [top.clientWidth + over, true];
    else if (compact && top.clientWidth >= need) compact = false;
  }
  $effect(() => {
    if (!top || typeof ResizeObserver === 'undefined') return;
    void title;
    const ro = new ResizeObserver(fit);
    ro.observe(top);
    return () => ro.disconnect();
  });
  // `title` may be a snippet for a custom heading. Editing: click saves on Enter or blur, cancels on Esc.
  let editing = $state(false);
  let draft = $state('');
  let box: HTMLElement | undefined = $state();
  const start = () => {
    draft = title as string;
    editing = true;
    queueMicrotask(() => box?.querySelector('input')?.select());
  };
  const finish = (save: boolean) => {
    if (!editing) return;
    editing = false;
    const v = draft.trim();
    if (save && v && v !== title) onrename?.(v);
  };
</script>

<header class="page-header">
  <div class="top" class:compact bind:this={top}>
    {#if iconEdit}{@render iconEdit()}{:else if icon}<EntityIcon {icon} {color} />{/if}
    <h1 bind:this={heading}>
      {#if typeof title !== 'string'}{@render title()}
      {:else if editable && editing}
        <span bind:this={box} class="edit">
          <TextInput label="Title" bind:value={draft} onblur={() => finish(true)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') finish(true); else if (e.key === 'Escape') finish(false); }} />
        </span>
      {:else if editable}<button type="button" class="name" data-title onclick={start}>{title}</button>
      {:else}{title}{/if}
    </h1>
    {#if tabs}<div class="tabs" bind:this={strip}>{@render tabs()}</div>{/if}
    <div class="grow"></div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
  {#if filters}<div class="filters">{@render filters()}</div>{/if}
</header>

<style>
  .page-header { flex: none; }
  .top { height: var(--topbar-h); display: flex; align-items: center; gap: var(--s-8); padding: 0 var(--s-7); min-width: 0; }
  h1 { font-size: var(--fs-title); font-weight: 700; letter-spacing: -0.02em; line-height: var(--lh-tight); min-width: 4em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .compact { gap: var(--s-6); }
  .compact :global(.cx-tab-label:not(:only-child)) { display: none; }
  .compact :global(.cx-tabs:not(.cx-seg) .cx-tab) { padding: 0 var(--s-3); }
  .compact :global(.actions .cmd-label) { display: none; }
  .name { border: 0; background: transparent; padding: 0; font: inherit; letter-spacing: inherit; color: inherit; cursor: text; }
  .edit { display: inline-block; min-width: 240px; font-size: var(--fs-md); }
  .tabs { align-self: stretch; display: flex; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .grow { flex: 1; }
  .actions { display: flex; align-items: center; gap: var(--s-4); flex: none; }
  .filters { height: 36px; display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-7); }
</style>

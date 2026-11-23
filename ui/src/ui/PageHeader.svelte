<script lang="ts">
  import type { Snippet } from 'svelte';
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
  }: { title: string | Snippet; editable?: boolean; onrename?: (value: string) => void; icon?: string; color?: string; tabs?: Snippet; filters?: Snippet; actions?: Snippet } = $props();
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
  <div class="top">
    {#if icon}<EntityIcon {icon} {color} />{/if}
    <h1>
      {#if typeof title !== 'string'}{@render title()}
      {:else if editable && editing}
        <span bind:this={box} class="edit">
          <TextInput label="Title" bind:value={draft} onblur={() => finish(true)} onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') finish(true); else if (e.key === 'Escape') finish(false); }} />
        </span>
      {:else if editable}<button type="button" class="name" data-title onclick={start}>{title}</button>
      {:else}{title}{/if}
    </h1>
    {#if tabs}<div class="tabs">{@render tabs()}</div>{/if}
    <div class="grow"></div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
  {#if filters}<div class="filters">{@render filters()}</div>{/if}
</header>

<style>
  .page-header { flex: none; }
  .top { height: var(--topbar-h); display: flex; align-items: center; gap: var(--s-8); padding: 0 var(--s-7); }
  h1 { font-size: var(--fs-title); font-weight: 700; letter-spacing: -0.02em; line-height: var(--lh-tight); }
  .name { border: 0; background: transparent; padding: 0; font: inherit; letter-spacing: inherit; color: inherit; cursor: text; }
  .edit { display: inline-block; min-width: 240px; font-size: var(--fs-md); }
  .tabs { align-self: stretch; display: flex; }
  .grow { flex: 1; }
  .actions { display: flex; align-items: center; gap: var(--s-4); }
  .filters { height: 36px; display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-7); }
</style>

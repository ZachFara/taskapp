<script lang="ts">
  import type { Snippet } from 'svelte';

  type Opt = { id: string | number; name: string };
  // A list of choices inside an editor popover. `none` adds a clearing row at the top.
  let { options, value, label, onpick, none, lead }: {
    options: Opt[];
    value: string | number | null;
    label: string;
    onpick: (id: string | number | null) => void;
    none?: string;
    lead?: Snippet<[Opt]>;
  } = $props();
</script>

<ul class="opts" role="listbox" aria-label={label}>
  {#if none}
    <li><button type="button" role="option" aria-selected={value === null} onclick={() => onpick(null)}><span class="lead"></span>{none}</button></li>
  {/if}
  {#each options as o (o.id)}
    <li><button type="button" role="option" aria-selected={value === o.id} onclick={() => onpick(o.id)}>
      <span class="lead">{#if lead}{@render lead(o)}{/if}</span>{o.name}</button></li>
  {/each}
</ul>

<style>
  .opts { list-style: none; margin: calc(var(--s-4) * -1); padding: var(--s-2); max-height: 280px; overflow-y: auto; }
  button { display: flex; align-items: center; gap: var(--s-5); width: 100%; height: var(--ctl-h); padding: 0 var(--s-4); border: 0; border-radius: var(--r-sm); background: transparent; text-align: left; font-weight: 500; }
  button:hover { background: var(--active); }
  button[aria-selected='true'] { background: var(--selection); font-weight: 700; }
  .lead { width: var(--icon); display: grid; place-items: center; }
</style>

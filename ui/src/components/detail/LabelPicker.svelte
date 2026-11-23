<script lang="ts">
  import type { Label } from '../../lib/bindings';
  import Checkbox from '../../ui/Checkbox.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  // Tick labels, find one by typing, or make a new one from what was typed (Enter).
  // `ondelete` asks to delete a label everywhere (the caller confirms first).
  let { labels, selected, ontoggle, oncreate, ondelete }: {
    labels: Label[];
    selected: number[];
    ontoggle: (id: number, on: boolean) => void;
    oncreate: (name: string) => void | Promise<void>;
    ondelete?: (label: Label) => void;
  } = $props();
  let text = $state('');
  const typed = $derived(text.trim());
  const shown = $derived(labels.filter((l) => l.name.toLowerCase().includes(typed.toLowerCase())));
  const exact = $derived(labels.find((l) => l.name.toLowerCase() === typed.toLowerCase()));
  const enter = async (e: KeyboardEvent) => {
    if (e.key !== 'Enter' || !typed) return;
    e.preventDefault();
    if (exact) ontoggle(exact.id, !selected.includes(exact.id));
    else await oncreate(typed);
    text = '';
  };
</script>

<div class="picker">
  <TextInput label="Find or create a label" placeholder="Find or create a label" bind:value={text} onkeydown={enter} />
  <ul class="checks" aria-label="Labels">
    {#each shown as l (l.id)}
      <li>
        <Checkbox label={l.name} bind:checked={() => selected.includes(l.id), (on) => ontoggle(l.id, on)} /> <span class="name">{l.name}</span>
        {#if ondelete}<IconButton icon="trash" label={`Delete label ${l.name}`} onclick={() => ondelete(l)} />{/if}
      </li>
    {/each}
    {#if typed && !exact}
      <li><button type="button" class="create" onclick={async () => { await oncreate(typed); text = ''; }}>Create label "{typed}"</button></li>
    {:else if !labels.length}
      <li class="none">No labels yet. Type a name and press Enter.</li>
    {/if}
  </ul>
</div>

<style>
  .picker { display: grid; gap: var(--s-4); min-width: 14rem; }
  .checks { list-style: none; display: grid; gap: var(--s-4); }
  .checks li { display: flex; align-items: center; gap: var(--s-4); }
  .name { flex: 1; min-width: 0; }
  .none { color: var(--muted); }
  .create { border: 0; background: transparent; padding: 0; font: inherit; color: var(--accent); cursor: pointer; text-align: left; }
  .create:hover { text-decoration: underline; }
</style>

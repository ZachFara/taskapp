<script lang="ts">
  import { onMount } from 'svelte';
  import { groups } from '../lib/commands';
  import Dialog from '../ui/Dialog.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import TextInput from '../ui/TextInput.svelte';

  // Opened by the help.shortcuts command. Every row comes from the registry, so a new command shows up here by itself.
  let { open = $bindable(false) }: { open?: boolean } = $props();
  let query = $state('');

  onMount(() => {
    const show = () => (open = true);
    window.addEventListener('command:help.shortcuts', show);
    return () => window.removeEventListener('command:help.shortcuts', show);
  });

  const needle = $derived(query.trim().toLowerCase());
  const sections = $derived(
    groups()
      .map(({ group, commands }) => ({
        group,
        commands: commands.filter((c) => !needle || c.label.toLowerCase().includes(needle) || group.toLowerCase().includes(needle)),
      }))
      .filter((s) => s.commands.length > 0),
  );
</script>

<Dialog bind:open title="Keyboard shortcuts">
  <div class="sheet">
    <TextInput bind:value={query} label="Filter shortcuts" placeholder="Filter shortcuts" type="search" />
    {#each sections as s (s.group)}
      <section>
        <h3>{s.group}</h3>
        <ul>
          {#each s.commands as c (c.id)}
            <li data-row>
              <span class="name">{c.label}</span>
              <span class="keys">
                {#each [c.shortcut ?? []].flat() as seq (seq)}
                  <span class="combo"><Kbd keys={seq.split(' ')} /></span>
                {/each}
              </span>
            </li>
          {/each}
        </ul>
      </section>
    {:else}
      <p class="empty">No shortcuts match "{query.trim()}"</p>
    {/each}
  </div>
</Dialog>

<style>
  .sheet { display: grid; gap: var(--s-5); columns: 2; column-gap: var(--s-6); }
  .sheet > :global(*) { break-inside: avoid; }
  section { display: grid; gap: var(--s-2); margin-bottom: var(--s-5); }
  h3 {
    margin: 0;
    padding: var(--s-2) var(--s-4);
    border-left: 3px solid var(--stone);
    background: var(--band-1);
    color: var(--band-ink);
    font-size: var(--fs-xs);
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  ul { list-style: none; margin: 0; padding: 0; }
  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-4);
    min-height: var(--row-h-compact);
    padding: 0 var(--s-4);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
  }
  .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .keys { display: flex; flex: none; gap: var(--s-3); }
  .combo { display: inline-flex; gap: var(--s-1); }
  .empty { margin: 0; color: var(--muted); }
</style>

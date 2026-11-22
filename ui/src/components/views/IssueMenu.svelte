<script lang="ts">
  import type { Snippet } from 'svelte';
  import { api } from '../../lib/api';
  import { listCommands } from '../../lib/commands';
  import { selection } from '../../lib/selection.svelte';
  import { navigate } from '../../app/routes';
  import ContextMenu from '../../ui/ContextMenu.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';

  // Right-click menu for one issue, built from the registry so it always matches the command menu.
  let { issueId, children }: { issueId: number; children: Snippet } = $props();
  let open = $state(false);
  const ctx = () => ({ route: location.pathname, selection: selection.selected, focusedIssueId: issueId, openIssueId: null, api, navigate });
  const items = $derived(open ? listCommands(ctx()).filter((c) => c.run) : []);
  $effect(() => {
    if (open) selection.focus(issueId);
  });
</script>

<ContextMenu bind:open>
  {#snippet area()}{@render children()}{/snippet}
  {#each items as c (c.id)}
    <MenuItem label={c.label} icon={c.icon} shortcut={c.shortcut} onSelect={() => c.run!(ctx())} />
  {/each}
</ContextMenu>

<style>
  :global(.cx-ctx-area) { display: contents; }
</style>

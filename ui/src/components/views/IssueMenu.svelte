<script lang="ts">
  import type { Snippet } from 'svelte';
  import { api } from '../../lib/api';
  import { listCommands } from '../../lib/commands';
  import { selection } from '../../lib/selection.svelte';
  import { navigate } from '../../app/routes';
  import ContextMenu from '../../ui/ContextMenu.svelte';
  import MenuItem from '../../ui/MenuItem.svelte';

  // One right-click menu around a whole list, board or table: the row under the pointer (`data-id`) picks the issue,
  // and the items come from the registry so they always match the command menu. One instance keeps big lists fast.
  let { children }: { children: Snippet } = $props();
  let open = $state(false);
  let issueId = $state<number | null>(null);
  const ctx = (id: number) => ({ route: location.pathname, selection: selection.selected, focusedIssueId: id, openIssueId: null, api, navigate });
  const items = $derived(open && issueId !== null ? listCommands(ctx(issueId)).filter((c) => c.run) : []);

  function aim(e: MouseEvent) {
    const row = (e.target as Element).closest<HTMLElement>('[data-id]');
    if (!row) return void e.stopPropagation();
    issueId = Number(row.dataset.id);
    selection.focus(issueId);
  }
</script>

<ContextMenu bind:open>
  {#snippet area()}<div class="fill" role="presentation" oncontextmenucapture={aim}>{@render children()}</div>{/snippet}
  {#each items as c (c.id)}
    <MenuItem label={c.label} icon={c.icon} shortcut={c.shortcut} onSelect={() => c.run!(ctx(issueId!))} />
  {/each}
</ContextMenu>

<style>
  :global(.cx-ctx-area) { display: contents; }
  .fill { height: 100%; }
</style>

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
  const ctx = (id: number) => ({ route: location.pathname, layout: selection.layout, selection: selection.selected, focusedIssueId: id, openIssueId: null, api, navigate });
  // Issue actions only, grouped: properties, then relations and copies, then pin and archive.
  const ORDER = [
    'list.title.edit',
    'issue.status', 'issue.priority', 'issue.labels', 'issue.project', 'issue.cycle', 'issue.start', 'issue.due', 'issue.estimate',
    'issue.relation', 'issue.parent', 'issue.duplicate', 'issue.copyKey', 'issue.copyLink', 'issue.next', 'issue.pin', 'issue.archive',
  ];
  const items = $derived.by(() => {
    if (!open || issueId === null) return [];
    const live = listCommands(ctx(issueId));
    return ORDER.flatMap((id) => live.filter((c) => c.id === id));
  });
  // A page command opens the command menu at its page, like its key does.
  const choose = (c: (typeof items)[number], id: number) =>
    c.run ? c.run(ctx(id)) : dispatchEvent(new CustomEvent('command:palette.open', { detail: { page: c.id } }));

  function aim(e: MouseEvent) {
    const row = (e.target as Element).closest<HTMLElement>('[data-id]');
    if (!row) return void e.stopPropagation();
    issueId = Number(row.dataset.id);
    if (!selection.selected.includes(issueId)) selection.clear();
    selection.focus(issueId);
  }
</script>

<ContextMenu bind:open>
  {#snippet area()}<div class="fill" role="presentation" oncontextmenucapture={aim}>{@render children()}</div>{/snippet}
  {#each items as c (c.id)}
    <MenuItem label={c.label} icon={c.icon} shortcut={c.shortcut} onSelect={() => choose(c, issueId!)} />
  {/each}
</ContextMenu>

<style>
  :global(.cx-ctx-area) { display: contents; }
  .fill { height: 100%; }
</style>

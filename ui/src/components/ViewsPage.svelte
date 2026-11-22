<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { SavedView } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import ListRow from '../ui/ListRow.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';

  let views = $state<SavedView[]>([]);
  let pinned = $state<Set<string>>(new Set());
  let loaded = $state(false);
  let doomed = $state<SavedView | null>(null);
  let confirming = $state(false);

  async function load() {
    const [list, pins] = await Promise.all([api.views.list(), api.pins.listResolved()]);
    views = list;
    pinned = new Set(pins.map((p) => p.route));
    loaded = true;
  }

  async function create() {
    const v = await api.views.save(null, 'Untitled view', { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }, null, null);
    navigate(`/view/${v.id}`);
  }

  async function remove() {
    if (!doomed) return;
    await api.views.delete(doomed.id);
    confirming = false;
    await load();
  }

  onMount(load);
</script>

<PageHeader title="Views" icon="icon:layers">
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={create}>New view</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && views.length === 0}
    <EmptyState title="No saved views" hint="A view is a filter, grouping and layout you come back to.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={create}>New view</Button>{/snippet}
    </EmptyState>
  {:else}
    <ul class="rows">
      {#each views as v (v.id)}
        <li class="row" data-view-id={v.id}>
          <ListRow onpick={() => navigate(`/view/${v.id}`)}>
            <EntityIcon icon={v.icon ?? 'icon:layers'} color={v.color} />
            <span class="name">{v.name}</span>
            <Pill>{v.def.layout}</Pill>
            {#if pinned.has(`/view/${v.id}`)}<Pill>Pinned</Pill>{/if}
          </ListRow>
          <IconButton icon="trash" label={`Delete ${v.name}`} onclick={() => ((doomed = v), (confirming = true))} />
        </li>
      {/each}
    </ul>
  {/if}
</main>

<Dialog bind:open={confirming} title="Delete view?" description={doomed ? `"${doomed.name}" goes away. Its issues stay.` : undefined}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (confirming = false)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete view</Button>
  {/snippet}
</Dialog>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: var(--s-3); padding-right: var(--s-4); border-bottom: 1px solid var(--border); }
  .name { font-weight: 600; margin-right: auto; }
</style>

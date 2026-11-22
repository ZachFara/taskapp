<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { openExternal } from '../lib/openExternal';
  import type { PinKind, ResolvedPin } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import EmptyState from '../ui/EmptyState.svelte';
  import GroupHeader from '../ui/GroupHeader.svelte';
  import Icon from '../ui/Icon.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';

  const kindLabels: Record<PinKind, string> = {
    Issue: 'Issues',
    Project: 'Projects',
    View: 'Views',
    Cycle: 'Cycles',
    Note: 'Notes',
    Url: 'Links',
  };
  // Legend band tone follows how far along the kind is: issues read deepest, links shallowest.
  const category: Record<PinKind, 'Backlog' | 'Unstarted' | 'Started'> = {
    Issue: 'Started',
    Project: 'Unstarted',
    View: 'Unstarted',
    Cycle: 'Unstarted',
    Note: 'Backlog',
    Url: 'Backlog',
  };
  const glyph: Record<PinKind, string> = { Issue: 'circle', Project: 'project', View: 'list', Cycle: 'cycle', Note: 'edit', Url: 'link' };

  let pins = $state<ResolvedPin[]>([]);
  let loaded = $state(false);
  let dragging = $state<number | null>(null);

  const groups = $derived(
    (Object.keys(kindLabels) as PinKind[])
      .map((kind) => ({ kind, label: kindLabels[kind], items: pins.filter((p) => p.pin.kind === kind) }))
      .filter((g) => g.items.length > 0),
  );

  async function load() {
    pins = await api.pins.listResolved();
    loaded = true;
  }

  async function drop(targetId: number, e: Event) {
    e.preventDefault();
    const from = dragging;
    dragging = null;
    if (from === null || from === targetId) return;
    const ids = pins.map((p) => p.pin.id);
    ids.splice(ids.indexOf(from), 1);
    ids.splice(ids.indexOf(targetId), 0, from);
    await api.pins.reorder(ids);
    await load();
  }

  async function unpin(id: number) {
    await api.pins.unpin(id);
    await load();
  }

  // Only http(s) URLs become links; a pinned Url with any other scheme is shown as plain text.
  const linkFor = (p: ResolvedPin) => (p.pin.kind !== 'Url' || /^https?:\/\//i.test(p.route) ? p.route : null);

  // A Url pin leaves the app through openExternal; a webview would ignore target=_blank.
  function open(e: MouseEvent, p: ResolvedPin) {
    e.preventDefault();
    if (p.pin.kind === 'Url') void openExternal(p.route);
    else navigate(p.route);
  }

  onMount(load);
</script>

<PageHeader title="Pinned" icon="icon:pin" />

<main class="body">
  {#if loaded && pins.length === 0}
    <EmptyState title="Nothing pinned yet" hint="Pin an issue, project or view to see it here." />
  {:else}
    {#each groups as group (group.kind)}
      <section data-kind={group.kind} class="group">
        <GroupHeader name={group.label} count={group.items.length} category={category[group.kind]} />
        <ul class="rows">
          {#each group.items as p (p.pin.id)}
            <li
              class="row"
              data-pin-id={p.pin.id}
              class:archived={p.archived}
              draggable="true"
              ondragstart={() => (dragging = p.pin.id)}
              ondragover={(e) => e.preventDefault()}
              ondrop={(e) => drop(p.pin.id, e)}
            >
              <Icon name={glyph[p.pin.kind]} />
              {#if linkFor(p)}
                <a href={linkFor(p)} target={p.pin.kind === 'Url' ? '_blank' : undefined} rel="noopener" onclick={(e) => open(e, p)}>{p.label}</a>
              {:else}
                <span>{p.label}</span>
              {/if}
              {#if p.archived}<span class="tag">Archived</span>{/if}
              <span class="grow"></span>
              <IconButton icon="close" label={`Unpin ${p.label}`} onclick={() => unpin(p.pin.id)} />
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  {/if}
</main>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); }
  .group { display: grid; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .row { display: flex; align-items: center; gap: var(--s-5); height: var(--row-h); padding: 0 var(--s-4) 0 var(--s-6); border-bottom: 1px solid var(--border); cursor: default; }
  .row:hover { background: var(--active); }
  .row a { color: var(--ink); text-decoration: none; font-weight: 500; }
  .row a:hover { text-decoration: underline; }
  .archived { opacity: 0.5; }
  .tag { font-size: var(--fs-xs); color: var(--muted); border: 1px solid var(--border); border-radius: var(--r-sm); padding: 0 var(--s-3); }
  .grow { flex: 1; }
</style>

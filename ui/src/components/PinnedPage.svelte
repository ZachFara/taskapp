<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { openExternal } from '../lib/openExternal';
  import type { PinKind, ResolvedPin } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import GroupHeader from '../ui/GroupHeader.svelte';
  import Icon from '../ui/Icon.svelte';
  import ListRow from '../ui/ListRow.svelte';
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

  // The root opens the first pinned view, else the first saved view; /pinned itself stays put.
  async function home() {
    if (location.pathname !== '/') return;
    const first = (await api.views.list())[0];
    const to = pins.find((p) => p.pin.kind === 'View')?.route ?? (first ? `/view/${first.id}` : null);
    if (!to) return;
    history.replaceState(null, '', to);
    dispatchEvent(new PopStateEvent('popstate'));
  }

  onMount(async () => {
    await load();
    await home();
  });
</script>

<PageHeader title="Pinned" icon="icon:pin" />

<main class="body">
  {#if loaded && pins.length === 0}
    <EmptyState title="Nothing pinned yet" hint="Pin an issue, project or view to see it here. Open one and press the pin.">
      {#snippet action()}<Button onclick={() => navigate('/views')}>Open a view</Button>{/snippet}
    </EmptyState>
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
              <ListRow onpick={() => (p.pin.kind === 'Url' ? linkFor(p) && void openExternal(p.route) : navigate(p.route))}>
                <Icon name={glyph[p.pin.kind]} />
                <span class="label">{p.label}</span>
                {#if p.archived}<span class="tag">Archived</span>{/if}
              </ListRow>
              {#if p.pin.kind === 'Url' && linkFor(p)}
                <a class="ext" href={linkFor(p)} target="_blank" rel="noopener" aria-label={`Open ${p.label}`} onclick={(e) => open(e, p)}><Icon name="link" /></a>
              {/if}
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
  .row { display: flex; align-items: center; gap: var(--s-3); padding-right: var(--s-4); border-bottom: 1px solid var(--border); }
  .label { font-weight: 500; }
  .ext { display: inline-flex; color: var(--muted); }
  .archived { opacity: 0.5; }
  .tag { font-size: var(--fs-xs); color: var(--muted); border: 1px solid var(--border); border-radius: var(--r-sm); padding: 0 var(--s-3); }
</style>

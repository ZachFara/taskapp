<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { PinKind, ResolvedPin } from '../lib/bindings';
  import { navigate } from '../app/routes';

  const kindLabels: Record<PinKind, string> = {
    Issue: 'Issues',
    Project: 'Projects',
    View: 'Views',
    Cycle: 'Cycles',
    Note: 'Notes',
    Url: 'Links',
  };

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

  // Only http(s) URLs become links; a pinned Url with any other scheme is shown as plain text.
  const linkFor = (p: ResolvedPin) => (p.pin.kind !== 'Url' || /^https?:\/\//i.test(p.route) ? p.route : null);

  function open(e: MouseEvent, p: ResolvedPin) {
    if (p.pin.kind === 'Url') return;
    e.preventDefault();
    navigate(p.route);
  }

  onMount(load);
</script>

{#if loaded && pins.length === 0}
  <h1>Pinned</h1>
  <p>Nothing pinned yet. Pin an issue, project or view to see it here.</p>
{:else}
  <h1>Pinned</h1>
  {#each groups as group (group.kind)}
    <section data-kind={group.kind}>
      <h2>{group.label}</h2>
      <ul>
        {#each group.items as p (p.pin.id)}
          <li
            data-pin-id={p.pin.id}
            class:archived={p.archived}
            draggable="true"
            ondragstart={() => (dragging = p.pin.id)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => drop(p.pin.id, e)}
          >
            {#if linkFor(p)}
              <a href={linkFor(p)} target={p.pin.kind === 'Url' ? '_blank' : undefined} rel="noopener" onclick={(e) => open(e, p)}>{p.label}</a>
            {:else}
              <span>{p.label}</span>
            {/if}
            {#if p.archived}<span class="tag">Archived</span>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
{/if}

<style>
  .archived {
    opacity: 0.5;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Cycle, PinKind, ResolvedPin } from '../lib/bindings';
  import { projects, refreshAll, views } from '../lib/stores.svelte';

  let pins = $state<ResolvedPin[]>([]);
  let cycles = $state<Cycle[]>([]);
  let error = $state('');

  onMount(async () => {
    const [, p, c] = await Promise.all([refreshAll(), api.pins.listResolved(), api.cycles.list()]);
    pins = p;
    cycles = c;
  });

  const fail = (e: unknown) => (error = (e as { message?: string }).message ?? String(e));
  const isPinned = (kind: PinKind, target: string) => pins.some((r) => r.pin.kind === kind && r.pin.target === target);

  async function toggle(kind: PinKind, target: string, title: string) {
    try {
      const have = pins.find((r) => r.pin.kind === kind && r.pin.target === target);
      if (have) await api.pins.unpin(have.pin.id);
      else await api.pins.pin(kind, target, title);
      pins = await api.pins.listResolved();
    } catch (e) {
      fail(e);
    }
  }

  // Never run a script URL from a pin; any other scheme (https, obsidian) goes to the system.
  const external = (route: string) => /^[a-z][a-z0-9+.-]*:/i.test(route);
  function open(route: string) {
    if (!external(route)) return navigate(route);
    if (!/^(javascript|data|vbscript):/i.test(route)) window.open(route, '_blank', 'noopener');
  }

  const moved = <T,>(ids: T[], from: T, to: T) => {
    const rest = ids.filter((i) => i !== from);
    rest.splice(ids.indexOf(to), 0, from);
    return rest;
  };
  let dragging: number | null = null;
  const drag = (id: number) => ({ draggable: true, ondragstart: () => (dragging = id), ondragover: (e: Event) => e.preventDefault() });

  async function dropPin(target: number) {
    if (dragging === null || dragging === target) return;
    const before = pins;
    const ids = moved(pins.map((r) => r.pin.id), dragging, target);
    pins = ids.map((id) => before.find((r) => r.pin.id === id)!);
    try {
      await api.pins.reorder(ids);
    } catch (e) {
      pins = before;
      fail(e);
    }
  }
  async function dropView(target: number) {
    if (dragging === null || dragging === target) return;
    try {
      await api.views.reorder(moved(views.map((v) => v.id), dragging, target));
      await refreshAll();
    } catch (e) {
      fail(e);
    }
  }
</script>

<nav aria-label="Sidebar">
  {#if error}<p role="alert" class="error">{error}</p>{/if}

  <section aria-label="Pinned">
    <h3>Pinned</h3>
    {#each pins as r (r.pin.id)}
      <div class="row" class:archived={r.archived} {...drag(r.pin.id)} ondrop={() => dropPin(r.pin.id)} role="listitem">
        <a href={r.route} onclick={(e) => (e.preventDefault(), open(r.route))}>{r.label}</a>
        <button class="hover" aria-label={`Unpin ${r.label}`} onclick={() => api.pins.unpin(r.pin.id).then(async () => (pins = await api.pins.listResolved()), fail)}>x</button>
      </div>
    {/each}
  </section>

  <section aria-label="Views">
    <h3>Views</h3>
    {#each views as v (v.id)}
      <div class="row" {...drag(v.id)} ondrop={() => dropView(v.id)} role="listitem">
        <a href={`/view/${v.id}`} onclick={(e) => (e.preventDefault(), navigate(`/view/${v.id}`))}>{v.name}</a>
        <button aria-label={`${isPinned('View', String(v.id)) ? 'Unpin' : 'Pin'} ${v.name}`} aria-pressed={isPinned('View', String(v.id))} onclick={() => toggle('View', String(v.id), v.name)}>*</button>
      </div>
    {/each}
  </section>

  <section aria-label="Projects">
    <h3>Projects</h3>
    {#each projects as p (p.id)}
      <div class="row">
        <a href={`/project/${p.slug}`} onclick={(e) => (e.preventDefault(), navigate(`/project/${p.slug}`))}>{p.name}</a>
        <button aria-label={`${isPinned('Project', p.slug) ? 'Unpin' : 'Pin'} ${p.name}`} aria-pressed={isPinned('Project', p.slug)} onclick={() => toggle('Project', p.slug, p.name)}>*</button>
      </div>
    {/each}
  </section>

  <section aria-label="Cycles">
    <h3>Cycles</h3>
    {#each cycles as c (c.id)}
      {@const label = c.name ?? `${c.start_date} to ${c.end_date}`}
      <div class="row">
        <a href="/cycles" onclick={(e) => (e.preventDefault(), navigate('/cycles'))}>{label}</a>
        <button aria-label={`${isPinned('Cycle', String(c.id)) ? 'Unpin' : 'Pin'} ${label}`} aria-pressed={isPinned('Cycle', String(c.id))} onclick={() => toggle('Cycle', String(c.id), label)}>*</button>
      </div>
    {/each}
  </section>

  <a class="settings" href="/settings/general" onclick={(e) => (e.preventDefault(), navigate('/settings/general'))}>Settings</a>
</nav>

<style>
  nav { display: flex; flex-direction: column; gap: 8px; font-size: 13px; padding: 8px; width: 220px; }
  h3 { font-size: 11px; text-transform: uppercase; color: #888; margin: 4px 0; }
  .row { display: flex; justify-content: space-between; align-items: center; }
  .row button { visibility: hidden; border: 0; background: none; cursor: pointer; }
  .row:hover button, .row button:focus, .row button[aria-pressed='true'] { visibility: visible; }
  .archived a { opacity: 0.5; }
  .error { color: #c0392b; }
  a { color: inherit; text-decoration: none; }
</style>

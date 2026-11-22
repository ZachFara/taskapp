<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { openExternal } from '../lib/openExternal';
  import type { Cycle, PinKind, ResolvedPin } from '../lib/bindings';
  import { projects, refreshAll, views } from '../lib/stores.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import Icon from '../ui/Icon.svelte';
  import ProjectSpark from './insights/ProjectSpark.svelte';

  // The app passes its live route; standalone the sidebar reads the address once.
  let { route = location.pathname }: { route?: string } = $props();

  let pins = $state<ResolvedPin[]>([]);
  let closed = $state<Record<string, boolean>>({});
  let cycles = $state<Cycle[]>([]);
  let error = $state('');
  let sparklines = $state(false);

  onMount(async () => {
    const [, p, c, sp] = await Promise.all([refreshAll(), api.pins.listResolved(), api.cycles.list(), api.settings.get('sidebar_sparklines').catch(() => null)]);
    pins = p;
    cycles = c;
    sparklines = sp === true;
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

  // Any scheme other than the openExternal allow-list (script, file) is refused there, so a pin never runs code.
  const external = (route: string) => /^[a-z][a-z0-9+.-]*:/i.test(route);
  function open(route: string) {
    if (!external(route)) return navigate(route);
    void openExternal(route);
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

<script module lang="ts">
  const label = (c: Cycle) => c.name ?? `${c.start_date} to ${c.end_date}`;
  // A view without its own icon shows its layout; the stored icon wins when set.
  const LAYOUT_ICON: Record<string, string> = { List: 'list', Board: 'board', Table: 'table', Gantt: 'timeline', Calendar: 'calendar' };
</script>

{#snippet head(id: string, title: string, count: number)}
  <button class="head" type="button" aria-expanded={!closed[id]} onclick={() => (closed[id] = !closed[id])}>
    <Icon name={closed[id] ? 'chevron-right' : 'chevron-down'} size={12} />
    <span>{title}</span>
    <em>{count}</em>
  </button>
{/snippet}

{#snippet pinBtn(kind: PinKind, target: string, title: string)}
  {@const on = isPinned(kind, target)}
  <button class="aff" class:on type="button" aria-label={`${on ? 'Unpin' : 'Pin'} ${title}`} aria-pressed={on} onclick={() => toggle(kind, target, title)}><Icon name="pin" size={12} /></button>
{/snippet}

<nav aria-label="Sidebar" data-on-frame>
  {#if error}<p role="alert" class="error">{error}</p>{/if}

  <ul class="top">
    {#each [['/inbox', 'inbox', 'Inbox'], ['/my', 'user', 'My issues']] as [href, icon, text] (href)}
      <li><a class="item" class:on={route === href} aria-current={route === href ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><Icon name={icon} /><span class="t">{text}</span></a></li>
    {/each}
  </ul>

  <section aria-label="Pinned">
    {@render head('pinned', 'Pinned', pins.length)}
    {#if !closed.pinned}
      <ul>
        {#each pins as r (r.pin.id)}
          <li class="row" class:archived={r.archived} {...drag(r.pin.id)} ondrop={() => dropPin(r.pin.id)}>
            <a class="item" class:on={route === r.route} aria-current={route === r.route ? 'page' : undefined} href={r.route} onclick={(e) => (e.preventDefault(), open(r.route))}><Icon name="pin" /><span class="t">{r.label}</span></a>
            <button class="aff" type="button" aria-label={`Unpin ${r.label}`} onclick={() => api.pins.unpin(r.pin.id).then(async () => (pins = await api.pins.listResolved()), fail)}><Icon name="close" size={12} /></button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-label="Views">
    {@render head('views', 'Views', views.length)}
    {#if !closed.views}
      <ul>
        {#each views as v (v.id)}
          {@const href = `/view/${v.id}`}
          <li class="row" {...drag(v.id)} ondrop={() => dropView(v.id)}>
            <a class="item" class:on={route.startsWith(href)} aria-current={route.startsWith(href) ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><EntityIcon icon={v.icon ?? `icon:${LAYOUT_ICON[v.def.layout] ?? 'list'}`} color={v.color} /><span class="t">{v.name}</span></a>
            {@render pinBtn('View', String(v.id), v.name)}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-label="Projects">
    {@render head('projects', 'Projects', projects.length)}
    {#if !closed.projects}
      <ul>
        <li class="row"><a class="item" class:on={route === '/projects'} aria-current={route === '/projects' ? 'page' : undefined} href="/projects" onclick={(e) => (e.preventDefault(), navigate('/projects'))}><Icon name="layers" /><span class="t">All projects</span></a></li>
        {#each projects as p (p.id)}
          {@const href = `/project/${p.slug}`}
          <li class="row">
            <a class="item" class:on={route === href} aria-current={route === href ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><EntityIcon icon={p.icon ?? 'icon:project'} color={p.color} /><span class="t">{p.name}</span>{#if sparklines}<ProjectSpark projectId={p.id} name={p.name} />{/if}</a>
            {@render pinBtn('Project', p.slug, p.name)}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-label="Cycles">
    {@render head('cycles', 'Cycles', cycles.length)}
    {#if !closed.cycles}
      <ul>
        <li class="row"><a class="item" class:on={route === '/cycles'} aria-current={route === '/cycles' ? 'page' : undefined} href="/cycles" onclick={(e) => (e.preventDefault(), navigate('/cycles'))}><Icon name="cycle" /><span class="t">All cycles</span></a></li>
        {#each cycles as c (c.id)}
          <li class="row">
            <a class="item" href="/cycles" onclick={(e) => (e.preventDefault(), navigate('/cycles'))}><Icon name="cycle" /><span class="t">{label(c)}</span></a>
            {@render pinBtn('Cycle', String(c.id), label(c))}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <ul class="bottom">
    <li><a class="item" class:on={route.startsWith('/settings')} aria-current={route.startsWith('/settings') ? 'page' : undefined} href="/settings/general" onclick={(e) => (e.preventDefault(), navigate('/settings/general'))}><Icon name="settings" /><span class="t">Settings</span></a></li>
  </ul>
</nav>

<style>
  nav { display: flex; flex-direction: column; gap: var(--s-2); min-height: 100%; padding: 0 var(--s-4) var(--s-4); color: var(--frame-ink); font-size: var(--fs-md); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--s-1); }
  .bottom { margin-top: auto; padding-top: var(--s-4); }
  .item { display: flex; align-items: center; gap: var(--s-5); flex: 1; min-width: 0; height: var(--ctl-h); padding: 0 var(--s-3); border-radius: var(--r-ctl); color: var(--frame-ink); text-decoration: none; font-weight: 500; transition: background var(--dur-1) var(--ease); }
  .item :global(svg) { flex: none; color: var(--frame-muted); }
  .item:hover { background: var(--frame-inset); }
  .item.on { background: var(--frame-raised); font-weight: 600; }
  .item.on :global(svg) { color: var(--frame-ink); }
  .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .head { display: flex; align-items: center; gap: var(--s-3); width: 100%; height: var(--ctl-h); margin-top: var(--s-4); padding: 0 var(--s-3); border: 0; border-radius: var(--r-ctl); background: transparent; color: var(--frame-muted); font: inherit; font-size: var(--fs-sm); font-weight: 600; text-align: left; }
  .head:hover { color: var(--frame-ink); }
  .head em { margin-left: auto; font-style: normal; font-weight: 500; }
  .row { display: flex; align-items: center; position: relative; }
  .aff { position: absolute; right: var(--s-2); display: grid; place-items: center; width: var(--ctl-h-sm); height: var(--ctl-h-sm); padding: 0; border: 0; border-radius: var(--r-ctl); background: var(--frame-raised); color: var(--frame-ink); opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  .row:hover .aff, .aff:focus-visible, .aff.on { opacity: 1; }
  .archived .t { opacity: 0.5; }
  .error { margin: 0; color: var(--signal); }
</style>

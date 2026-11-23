<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { openExternal } from '../lib/openExternal';
  import type { Cycle, PinKind, ResolvedPin } from '../lib/bindings';
  import { projects, refreshAll, views } from '../lib/stores.svelte';
  import Icon from '../ui/Icon.svelte';
  import RowActions from '../ui/RowActions.svelte';
  import { rowActs, rowUi } from '../lib/rowUi.svelte';
  import { patch } from '../lib/rowActions';
  import Tooltip from '../ui/Tooltip.svelte';
  import { shortcutFor } from '../lib/commands';
  import ProjectSpark from './insights/ProjectSpark.svelte';
  import IconColorPicker from './pickers/IconColorPicker.svelte';

  // The app passes its live route; standalone the sidebar reads the address once.
  let { route = location.pathname }: { route?: string } = $props();

  let pins = $state<ResolvedPin[]>([]);
  let closed = $state<Record<string, boolean>>({});
  let cycles = $state<Cycle[]>([]);
  let error = $state('');
  let sparklines = $state(false);
  let picking = $state<string | null>(null);

  onMount(async () => {
    const [, p, c, sp] = await Promise.all([refreshAll(), api.pins.listResolved(), api.cycles.list(), api.settings.get('sidebar_sparklines').catch(() => null)]);
    pins = p;
    cycles = c;
    sparklines = sp === true;
  });

  // Row actions (pin, rename, delete) change pins and cycles; reload them when one lands.
  $effect(() => {
    const reload = async () => {
      pins = await api.pins.listResolved();
      cycles = await api.cycles.list();
    };
    addEventListener('rows:changed', reload);
    return () => removeEventListener('rows:changed', reload);
  });
  const fail = (e: unknown) => (error = (e as { message?: string }).message ?? String(e));
  const isPinned = (kind: PinKind, target: string) => pins.some((r) => r.pin.kind === kind && r.pin.target === target);

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
  <Tooltip label={`${closed[id] ? 'Expand' : 'Collapse'} ${title}`}>
    {#snippet children(props)}
      <button {...props} class="head" type="button" aria-expanded={!closed[id]} onclick={() => (closed[id] = !closed[id])}>
        <Icon name={closed[id] ? 'chevron-right' : 'chevron-down'} size={12} />
        <span>{title}</span>
        <em>{count}</em>
      </button>
    {/snippet}
  </Tooltip>
{/snippet}

{#snippet navLink(href: string, icon: string, text: string, id: string, on: boolean)}
  <Tooltip label={text} shortcut={shortcutFor(id)}>
    {#snippet children(props)}
      <a {...props} class="item" class:on aria-current={on ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><Icon name={icon} /><span class="t">{text}</span></a>
    {/snippet}
  </Tooltip>
{/snippet}

<nav aria-label="Sidebar" data-on-frame>
  {#if error}<p role="alert" class="error">{error}</p>{/if}

  <ul class="top">
    <li>{@render navLink('/my', 'user', 'My issues', 'nav.my', route === '/my')}</li>
  </ul>

  <section aria-label="Pinned">
    {@render head('pinned', 'Pinned', pins.length)}
    {#if !closed.pinned}
      <ul>
        {#each pins as r (r.pin.id)}
          <li class="row" class:archived={r.archived} {...drag(r.pin.id)} ondrop={() => dropPin(r.pin.id)}>
            <a class="item" class:on={route === r.route} aria-current={route === r.route ? 'page' : undefined} href={r.route} onclick={(e) => (e.preventDefault(), open(r.route))}><Icon name="pin" /><span class="t">{r.label}</span></a>
            <Tooltip label={`Unpin ${r.label}`}>
              {#snippet children(props)}
                <button {...props} class="aff" type="button" aria-label={`Unpin ${r.label}`} onclick={() => api.pins.unpin(r.pin.id).then(async () => (pins = await api.pins.listResolved()), fail)}><Icon name="close" size={12} /></button>
              {/snippet}
            </Tooltip>
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
            <RowActions label={v.name} actions={rowActs({ kind: 'view', id: v.id, name: v.name }, isPinned('View', String(v.id)))}>
              <IconColorPicker label={`Icon and colour of ${v.name}`} value={{ icon: v.icon ?? `icon:${LAYOUT_ICON[v.def.layout] ?? 'list'}`, color: v.color }} onchange={(c) => patch({ kind: 'view', id: v.id, name: v.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `view:${v.id}`, (o) => (rowUi.picking = o ? `view:${v.id}` : null)} />
              <a class="item" class:on={route.startsWith(href)} aria-current={route.startsWith(href) ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><span class="t">{v.name}</span></a>
            </RowActions>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-label="Projects">
    {@render head('projects', 'Projects', projects.length)}
    {#if !closed.projects}
      <ul>
        <li class="row">{@render navLink('/projects', 'project', 'All projects', 'nav.projects', route === '/projects')}</li>
        {#each projects as p (p.id)}
          {@const href = `/project/${p.slug}`}
          <li class="row">
            <RowActions label={p.name} actions={rowActs({ kind: 'project', id: p.id, name: p.name }, isPinned('Project', p.slug))}>
              <IconColorPicker label={`Icon and colour of ${p.name}`} value={{ icon: p.icon ?? 'icon:project', color: p.color }} onchange={(c) => patch({ kind: 'project', id: p.id, name: p.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `project:${p.id}`, (o) => (rowUi.picking = o ? `project:${p.id}` : null)} />
              <a class="item" class:on={route === href} aria-current={route === href ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><span class="t">{p.name}</span>{#if sparklines}<ProjectSpark projectId={p.id} name={p.name} />{/if}</a>
            </RowActions>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section aria-label="Cycles">
    {@render head('cycles', 'Cycles', cycles.length)}
    {#if !closed.cycles}
      <ul>
        <li class="row">{@render navLink('/cycles', 'cycle', 'All cycles', 'nav.cycles', route === '/cycles')}</li>
        {#each cycles as c (c.id)}
          <li class="row">
            <RowActions label={label(c)} actions={rowActs({ kind: 'cycle', id: c.id, name: label(c) }, isPinned('Cycle', String(c.id)))}>
              <IconColorPicker label={`Icon and colour of ${label(c)}`} value={{ icon: c.icon ?? 'icon:cycle', color: c.color }} onchange={(x) => patch({ kind: 'cycle', id: c.id, name: label(c) }, { icon: x.icon, color: x.color })} bind:open={() => rowUi.picking === `cycle:${c.id}`, (o) => (rowUi.picking = o ? `cycle:${c.id}` : null)} />
              <a class="item" href="/cycles" onclick={(e) => (e.preventDefault(), navigate('/cycles'))}><span class="t">{label(c)}</span></a>
            </RowActions>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <ul class="top">
    <li>{@render navLink('/events', 'calendar', 'Events', 'nav.events', route === '/events' || route.startsWith('/event/'))}</li>
  </ul>

  <ul class="bottom">
    <li>{@render navLink('/settings/general', 'settings', 'Settings', 'nav.settings', route.startsWith('/settings'))}</li>
  </ul>
</nav>

<style>
  nav { --acts-bg: var(--frame-inset); display: flex; flex-direction: column; gap: var(--s-2); min-height: 100%; padding: 0 var(--s-4) var(--s-4); color: var(--frame-ink); font-size: var(--fs-md); }
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

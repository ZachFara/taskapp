<script lang="ts">
  import { onMount } from 'svelte';
  import Collapse from '../ui/Collapse.svelte';
  import { cubicOut } from 'svelte/easing';
  import { slide } from 'svelte/transition';
  import { ms } from '../lib/motion.svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { openExternal } from '../lib/openExternal';
  import type { Snippet } from 'svelte';
  import type { Cycle, FolderSection, PinKind, Project, ResolvedPin, SavedView, ViewFolder } from '../lib/bindings';
  import { folders, placements, projects, projectStatuses, refreshAll, views } from '../lib/stores.svelte';
  import { buildRows, canNest, type FolderRow, type TreeRow } from '../lib/folderTree';
  import { dnd, dragRows, type Drop, type Src } from '../lib/pointerDrag.svelte';
  import type { Move } from '../lib/rowActions';
  import ContextMenu from '../ui/ContextMenu.svelte';
  import { glyphCategory } from '../lib/projectFields';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import Icon from '../ui/Icon.svelte';
  import RowActions from '../ui/RowActions.svelte';
  import { rowActs, rowUi } from '../lib/rowUi.svelte';
  import { patch } from '../lib/rowActions';
  import Tooltip from '../ui/Tooltip.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import { runRow } from '../lib/rowUi.svelte';
  import { sidebarPrefs } from '../lib/sidebarPrefs.svelte';
  import { decisions, openDecisions, watchDecisions } from '../lib/decisions.svelte';
  import { labelFor } from '../lib/commands';
  import { shortcutFor } from '../lib/commands';
  import { dashboard, loadDashboard } from './dashboard/dashboard.svelte';
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

  onMount(watchDecisions);
  onMount(loadDashboard);
  const waiting = $derived(openDecisions());
  const blocking = $derived(waiting.filter((d) => d.mode === 'blocking').length);

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
    const refused = (e: Event) => fail({ message: (e as CustomEvent<string>).detail });
    addEventListener('rows:changed', reload);
    addEventListener('rows:failed', refused);
    return () => (removeEventListener('rows:changed', reload), removeEventListener('rows:failed', refused));
  });
  const fail = (e: unknown) => (error = (e as { message?: string }).message ?? String(e));
  const isPinned = (kind: PinKind, target: string) => pins.some((r) => r.pin.kind === kind && r.pin.target === target);

  // Any scheme other than the openExternal allow-list (script, file) is refused there, so a pin never runs code.
  const external = (route: string) => /^[a-z][a-z0-9+.-]*:/i.test(route);
  function open(route: string) {
    if (!external(route)) return navigate(route);
    void openExternal(route);
  }

  // Folders stay open or closed per device: the choice is kept in this browser only.
  const FOLDERS_KEY = 'folders.closed';
  const read = (): Record<number, boolean> => {
    try {
      return JSON.parse(localStorage.getItem(FOLDERS_KEY) ?? '{}');
    } catch {
      return {};
    }
  };
  let shut = $state<Record<number, boolean>>(read());
  function setShut(id: number, to: boolean) {
    shut[id] = to;
    try {
      localStorage.setItem(FOLDERS_KEY, JSON.stringify(shut));
    } catch {
      /* private window: the folder still opens, it just is not remembered */
    }
  }

  // Each section is a tree: folders (nested) and the items placed in them.
  const tree = <T extends { id: number }>(section: FolderSection, items: T[]) => buildRows(section, folders, placements, items, { shut, placed: true });
  const pinRows = $derived(tree('pin', pins.map((r) => ({ id: r.pin.id, r }))));
  // Issue views and project views share the section and its folders.
  const viewRows = $derived(tree('view', views));
  const projectRows = $derived(tree('project', projects));
  const cycleRows = $derived(tree('cycle', cycles));

  /** The rows inside a folder, as the real tree draws them (a shut folder has none), and the folder's own depth. */
  function contents(f: ViewFolder) {
    const items = f.section === 'view' ? views : f.section === 'project' ? projects : f.section === 'cycle' ? cycles : [];
    const all = buildRows(f.section, folders, placements, items, { shut, placed: true });
    const at = all.findIndex((x) => x.type === 'folder' && x.folder.id === f.id);
    const rest = at < 0 ? [] : all.slice(at + 1);
    const end = rest.findIndex((x) => x.depth <= all[at].depth);
    return { inner: end < 0 ? rest : rest.slice(0, end), base: at < 0 ? 0 : all[at].depth };
  }

  /** What the drag needs to know about a row: what it is, where it sits, and the line or highlight to draw. */
  const attrs = (section: FolderSection, type: 'item' | 'folder', id: number, label: string, r: { container: number | null; pos: number; depth: number }, open = false) => ({
    'data-drag': type,
    'data-section': section,
    'data-id': id,
    'data-container': r.container ?? '',
    'data-pos': r.pos,
    'data-label': label,
    'data-open': open ? '' : undefined,
    'data-line': dnd.drop?.section === section && dnd.drop.line?.type === type && dnd.drop.line.id === id ? dnd.drop.line.edge : undefined,
    'data-into': type === 'folder' && dnd.drop?.section === section && dnd.drop.highlight === id ? '' : undefined,
    style: `--depth: ${r.depth}`,
  });
  const dragOpts = { onDrop: (s: Src, d: Drop) => runRow('folder.move', { kind: s.type === 'folder' ? 'folder' : s.section, id: s.id, name: s.label, folder: d.container, index: d.index } satisfies Move), open: (id: number) => setShut(id, false), fits: (id: number, to: number | null) => canNest(folders, id, to) };
</script>

<script module lang="ts">
  const label = (c: Cycle) => c.name ?? `${c.start_date} to ${c.end_date}`;
  // A view without its own icon shows its layout; the stored icon wins when set.
  const LAYOUT_ICON: Record<string, string> = { List: 'list', Board: 'board', Table: 'table', Gantt: 'timeline', Calendar: 'calendar' };
</script>

{#snippet head(id: string, title: string, count: number, section: FolderSection)}
  {#snippet bar()}
  <div class="row">
    <Tooltip label={`${closed[id] ? 'Expand' : 'Collapse'} ${title}`}>
      {#snippet children(props)}
        <button {...props} class="head" type="button" aria-expanded={!closed[id]} onclick={() => (closed[id] = !closed[id])}>
          <span class="chev" data-collapsed={!!closed[id]}><Icon name="chevron-down" size={12} /></span>
          <span>{title}</span>
          <em>{count}</em>
        </button>
      {/snippet}
    </Tooltip>
    <Menu>
      {#snippet trigger(props)}
        <button {...props} class="aff" type="button" aria-label={`${title} menu`}><Icon name="more" size={12} /></button>
      {/snippet}
      <MenuItem label={labelFor('folder.new') ?? 'New folder'} icon="folder" onSelect={() => runRow('folder.new', { section })} />
      {#if section === 'view'}<MenuItem label={labelFor('view.newProject') ?? 'New project view'} icon="layers" onSelect={() => runRow('view.newProject', undefined)} />{/if}
    </Menu>
  </div>
  {/snippet}
  {#if section === 'project'}
    <!-- Right-click on the Projects header: list every project here or only link "All projects". -->
    <ContextMenu>
      {#snippet area()}{@render bar()}{/snippet}
      <MenuItem label="Show projects in the sidebar" icon={sidebarPrefs.showProjects ? 'check' : undefined} onSelect={() => runRow('sidebar.showProjects', undefined)} />
    </ContextMenu>
  {:else}{@render bar()}{/if}
{/snippet}

{#snippet navLink(href: string, icon: string, text: string, id: string, on: boolean, count?: { n: number; urgent: number })}
  <Tooltip label={text} shortcut={shortcutFor(id)}>
    {#snippet children(props)}
      <a {...props} class="item" class:on aria-current={on ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><Icon name={icon} /><span class="t">{text}</span>{#if count}<em class="n" data-urgent={count.urgent > 0 || undefined} aria-label={`${count.n} open${count.urgent ? `, ${count.urgent} blocking` : ''}`}>{count.n}</em>{/if}</a>
    {/snippet}
  </Tooltip>
{/snippet}

{#snippet folderRow(r: FolderRow, section: FolderSection)}
  {@const f = r.folder}
  <li class="row" transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }} {...attrs(section, 'folder', f.id, f.name, r, !r.shut)}>
    <RowActions button={false} label={f.name} actions={rowActs({ kind: 'folder', id: f.id, name: f.name }, isPinned('Folder', String(f.id)))}>
      <button class="item folder" type="button" aria-expanded={!r.shut} onclick={() => setShut(f.id, !r.shut)}><span class="chev" data-collapsed={r.shut}><Icon name="chevron-down" size={12} /></span><span class="t">{f.name}</span><em>{r.count}</em></button>
    </RowActions>
  </li>
{/snippet}

<!-- One section's rows. `leaf` draws an item's row; the blank space below takes the right-click menu and the drop to the top level. -->
{#snippet rowsOf(section: FolderSection, rows: TreeRow<any>[], leaf: Snippet<[any, TreeRow<any>]>, lead?: Snippet, tail?: Snippet)}
  <ul>
    {@render lead?.()}
    {#each rows as r (r.type === 'folder' ? `f${r.folder.id}` : `i${r.item.id}`)}
      {#if r.type === 'folder'}{@render folderRow(r, section)}{:else}{@render leaf(r.item, r)}{/if}
    {/each}
    {@render tail?.()}
  </ul>
  <ContextMenu>
    {#snippet area()}
      <div class="blank" data-area data-section={section} data-over={dnd.drop?.section === section && dnd.drop.container === null && !dnd.drop.line ? '' : undefined}>{#if dnd.src?.section === section}Move to top level{/if}</div>
    {/snippet}
    <MenuItem label={labelFor('folder.new') ?? 'New folder'} icon="folder" onSelect={() => runRow('folder.new', { section })} />
  </ContextMenu>
{/snippet}

<!-- A pinned folder: its own row, then the live contents of the folder (the same rows and open state as in its section). -->
{#snippet pinnedFolder(r: ResolvedPin, row: TreeRow<unknown>)}
  {@const id = Number(r.pin.target)}
  {@const f = folders.find((x) => x.id === id)}
  {#if f}
    {@const { inner, base } = contents(f)}
    <li class="row" {...attrs('pin', 'item', r.pin.id, r.label, row)}>
      <RowActions button={false} label={`${r.label} (pinned)`} actions={rowActs({ kind: 'pin', id: r.pin.id, name: r.label })}>
        <button class="item folder" type="button" aria-expanded={!shut[id]} onclick={() => setShut(id, !shut[id])}><span class="chev" data-collapsed={!!shut[id]}><Icon name="chevron-down" size={12} /></span><Icon name="pin" /><span class="t">{r.label}</span></button>
      </RowActions>
    </li>
    {#each inner as x (x.type === 'folder' ? `f${x.folder.id}` : `i${x.item.id}`)}
      {@const rel = { ...x, depth: x.depth - base }}
      {#if x.type === 'folder'}{@render folderRow(rel as FolderRow, f.section)}
      {:else if f.section === 'view'}{@render viewLeaf(x.item as SavedView, rel)}
      {:else if f.section === 'project'}{@render projectLeaf(x.item as Project, rel)}
      {:else}{@render cycleLeaf(x.item as Cycle, rel)}{/if}
    {/each}
  {/if}
{/snippet}

{#snippet pinLeaf(p: { r: ResolvedPin }, row: TreeRow<unknown>)}
  {@const r = p.r}
  {#if r.pin.kind === 'Folder'}{@render pinnedFolder(r, row)}{:else}
  <li class="row" class:archived={r.archived} transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }} {...attrs('pin', 'item', r.pin.id, r.label, row)}>
    <RowActions button={false} label={`${r.label} (pinned)`} actions={rowActs({ kind: 'pin', id: r.pin.id, name: r.label })}>
      <a class="item" class:on={route === r.route} aria-current={route === r.route ? 'page' : undefined} href={r.route} onclick={(e) => (e.preventDefault(), open(r.route))}><Icon name="pin" /><span class="t">{r.label}</span></a>
    </RowActions>
  </li>
  {/if}
{/snippet}

{#snippet viewLeaf(v: SavedView, row: TreeRow<unknown>)}
  {@const href = `/view/${v.id}`}
  <li class="row" transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }} {...attrs('view', 'item', v.id, v.name, row)}>
    <RowActions button={false} label={v.name} actions={rowActs({ kind: 'view', id: v.id, name: v.name }, isPinned('View', String(v.id)))}>
      <IconColorPicker label={`Icon and colour of ${v.name}`} value={{ icon: v.icon ?? `icon:${LAYOUT_ICON[v.def.layout] ?? 'list'}`, color: v.color }} onchange={(c) => patch({ kind: 'view', id: v.id, name: v.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `view:${v.id}`, (o) => (rowUi.picking = o ? `view:${v.id}` : null)} />
      <a class="item" class:on={route.startsWith(href)} aria-current={route.startsWith(href) ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><span class="t">{v.name}</span>{#if v.entity === 'project'}<span class="st" role="img" aria-label="Project view"><Icon name="project" size={12} /></span>{/if}</a>
    </RowActions>
  </li>
{/snippet}

{#snippet projectLeaf(p: Project, row: TreeRow<unknown>)}
  {@const href = `/project/${p.slug}`}
  {@const st = projectStatuses.find((x) => x.id === p.status_id)}
  <li class="row" transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }} {...attrs('project', 'item', p.id, p.name, row)}>
    <RowActions button={false} label={p.name} actions={rowActs({ kind: 'project', id: p.id, name: p.name }, isPinned('Project', p.slug))}>
      <IconColorPicker label={`Icon and colour of ${p.name}`} value={{ icon: p.icon ?? 'icon:project', color: p.color }} onchange={(c) => patch({ kind: 'project', id: p.id, name: p.name }, { icon: c.icon, color: c.color })} bind:open={() => rowUi.picking === `project:${p.id}`, (o) => (rowUi.picking = o ? `project:${p.id}` : null)} />
      <a class="item" class:on={route === href} aria-current={route === href ? 'page' : undefined} {href} onclick={(e) => (e.preventDefault(), navigate(href))}><span class="t">{p.name}</span>{#if st}<span class="st"><StatusGlyph category={glyphCategory[st.category]} size={12} label={st.name} /></span>{/if}{#if sparklines}<ProjectSpark projectId={p.id} name={p.name} />{/if}</a>
    </RowActions>
  </li>
{/snippet}

{#snippet cycleLeaf(c: Cycle, row: TreeRow<unknown>)}
  <li class="row" transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }} {...attrs('cycle', 'item', c.id, label(c), row)}>
    <RowActions button={false} label={label(c)} actions={rowActs({ kind: 'cycle', id: c.id, name: label(c) }, isPinned('Cycle', String(c.id)))}>
      <IconColorPicker label={`Icon and colour of ${label(c)}`} value={{ icon: c.icon ?? 'icon:cycle', color: c.color }} onchange={(x) => patch({ kind: 'cycle', id: c.id, name: label(c) }, { icon: x.icon, color: x.color })} bind:open={() => rowUi.picking === `cycle:${c.id}`, (o) => (rowUi.picking = o ? `cycle:${c.id}` : null)} />
      <a class="item" href="/cycles" onclick={(e) => (e.preventDefault(), navigate('/cycles'))}><span class="t">{label(c)}</span></a>
    </RowActions>
  </li>
{/snippet}

{#snippet allProjects()}<li class="row">{@render navLink('/projects', 'project', 'All projects', 'nav.projects', route === '/projects')}</li>{/snippet}
{#snippet allCycles()}<li class="row">{@render navLink('/cycles', 'cycle', 'All cycles', 'nav.cycles', route === '/cycles')}</li>{/snippet}

<!-- Pointer drag, not HTML5 drag: the Mac app's web view does not deliver drop events reliably. -->
<nav aria-label="Sidebar" data-on-frame use:dragRows={dragOpts} ondragstart={(e) => e.preventDefault()}>
  {#if error}<p role="alert" class="error">{error}</p>{/if}

  <ul class="top">
    <li>{@render navLink('/my', 'user', 'My issues', 'nav.my', route === '/my')}</li>
  </ul>

  <section aria-label="Pinned">
    {@render head('pinned', 'Pinned', pins.length, 'pin')}
    <Collapse open={!closed.pinned}>{@render rowsOf('pin', pinRows, pinLeaf)}</Collapse>
  </section>

  <section aria-label="Views">
    {@render head('views', 'Views', views.length, 'view')}
    <Collapse open={!closed.views}>{@render rowsOf('view', viewRows, viewLeaf)}</Collapse>
  </section>

  <section aria-label="Projects">
    {@render head('projects', 'Projects', projects.length, 'project')}
    <Collapse open={!closed.projects}>
      {#if sidebarPrefs.showProjects}{@render rowsOf('project', projectRows, projectLeaf, allProjects)}{:else}<ul>{@render allProjects()}</ul>{/if}
    </Collapse>
  </section>

  <section aria-label="Cycles">
    {@render head('cycles', 'Cycles', cycles.length, 'cycle')}
    <Collapse open={!closed.cycles}>{@render rowsOf('cycle', cycleRows, cycleLeaf, allCycles)}</Collapse>
  </section>

  <ul class="top">
    {#if dashboard.on}<li>{@render navLink('/dashboard', 'chart', 'Dashboard', 'nav.dashboard', route === '/dashboard')}</li>{/if}
    <li>{@render navLink('/events', 'calendar', 'Events', 'nav.events', route === '/events' || route.startsWith('/event/'))}</li>
    <li>{@render navLink('/decisions', 'help', 'Decisions', 'nav.decisions', route === '/decisions', waiting.length > 0 && !decisions.error ? { n: waiting.length, urgent: blocking } : undefined)}</li>
  </ul>

  <ul class="bottom">
    <li>{@render navLink('/settings/general', 'settings', 'Settings', 'nav.settings', route.startsWith('/settings'))}</li>
  </ul>
</nav>

<style>
  .chev { display: flex; transition: transform var(--dur-2) var(--ease); }
  .chev[data-collapsed='true'] { transform: rotate(-90deg); }
  nav { display: flex; flex-direction: column; gap: var(--s-2); min-height: 100%; padding: 0 var(--s-4) var(--s-4); color: var(--frame-ink); font-size: var(--fs-md); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--s-1); }
  .bottom { margin-top: auto; padding-top: var(--s-4); }
  .item { display: flex; align-items: center; gap: var(--s-5); flex: 1; min-width: 0; height: var(--ctl-h); padding: 0 var(--s-3); border-radius: var(--r-ctl); color: var(--frame-ink); text-decoration: none; font-weight: 500; transition: background var(--dur-1) var(--ease); }
  .item :global(svg) { flex: none; color: var(--frame-muted); }
  .item:hover { background: var(--frame-inset); }
  .item.on { background: var(--frame-raised); font-weight: 600; }
  .item.on :global(svg) { color: var(--frame-ink); }
  .st { margin-left: auto; display: inline-flex; }
  .n { margin-left: auto; font-style: normal; font-size: var(--fs-xs); font-weight: 600; padding: 0 var(--s-3); border-radius: var(--r-round); background: var(--frame-inset); color: var(--frame-muted); }
  .n[data-urgent] { background: var(--c-red); color: var(--accent-ink); }
  .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  :global(.dnd-ghost) { position: fixed; left: 0; top: 0; z-index: var(--z-tooltip); pointer-events: none; padding: var(--s-2) var(--s-5); border-radius: var(--r-ctl); background: var(--frame-raised); color: var(--frame-ink); font-size: var(--fs-md); font-weight: 500; box-shadow: var(--shadow-float); }
  :global(body.dnd-active) { user-select: none; cursor: grabbing; }
  .head { display: flex; flex: 1; align-items: center; gap: var(--s-3); width: 100%; height: var(--ctl-h); margin-top: var(--s-4); padding: 0 var(--s-3); border: 0; border-radius: var(--r-ctl); background: transparent; color: var(--frame-muted); font: inherit; font-size: var(--fs-sm); font-weight: 600; text-align: left; }
  .head:hover { color: var(--frame-ink); }
  .head em { margin-left: auto; font-style: normal; font-weight: 500; }
  .row { display: flex; align-items: center; position: relative; }
  .aff { position: absolute; right: var(--s-2); display: grid; place-items: center; width: var(--ctl-h-sm); height: var(--ctl-h-sm); padding: 0; border: 0; border-radius: var(--r-ctl); background: var(--frame-raised); color: var(--frame-ink); opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  .row:hover .aff, .aff:focus-visible, .aff.on { opacity: 1; }
  /* Nested rows step in by one level each; the drag marks the target with a line or a highlight. */
  .row[data-drag] { padding-left: calc(var(--depth, 0) * var(--s-7)); }
  .row[data-line]::before { content: ''; position: absolute; left: 0; right: 0; height: 2px; background: var(--frame-ink); pointer-events: none; }
  .row[data-line='before']::before { top: calc(var(--s-1) / -2 - 1px); }
  .row[data-line='after']::before { bottom: calc(var(--s-1) / -2 - 1px); }
  .row[data-into] .item { background: var(--frame-raised); outline: 1px solid var(--frame-ink); }
  .blank { min-height: var(--s-4); border-radius: var(--r-ctl); color: var(--frame-muted); font-size: var(--fs-sm); text-align: center; }
  .blank:not(:empty) { display: grid; place-items: center; height: var(--ctl-h); border: 1px dashed var(--frame-muted); }
  .blank[data-over] { background: var(--frame-raised); color: var(--frame-ink); border-color: var(--frame-ink); }
  .folder { border: 0; background: transparent; font: inherit; text-align: left; cursor: default; }
  .folder em { margin-left: auto; font-style: normal; color: var(--frame-muted); }
  .archived .t { opacity: 0.5; }
  .error { margin: 0; color: var(--signal); }
</style>

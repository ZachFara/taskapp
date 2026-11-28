<script lang="ts">
  import { onMount, setContext, tick, untrack } from 'svelte';
  import { matchRoute, navigate } from './routes';
  import { FEATURES, isOn, ROUTES } from '../lib/features.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import { api } from '../lib/api';
  import { attachKeys, type Ctx, type Layout } from '../lib/commands';
  import { selection } from '../lib/selection.svelte';
  import { issues, projects, refreshAll, selectedIssueId, views } from '../lib/stores.svelte';
  import { viewContext } from '../lib/viewContext.svelte';
  import { defaultsFromFilter, mergeIssueDefaults, type IssueDefaults } from '../components/filters/defaults';
  import { initTheme } from '../lib/theme.svelte';
  import CheatSheet from '../components/CheatSheet.svelte';
  import CommandPalette from '../components/CommandPalette.svelte';
  import IssueDetail from '../components/IssueDetail.svelte';
  import QuickAdd from '../components/QuickAdd.svelte';
  import RestoreDialog from '../components/RestoreDialog.svelte';
  import Setup from '../components/Setup.svelte';
  import Shell from '../components/Shell.svelte';
  import Sidebar from '../components/Sidebar.svelte';
  import ImageViewer from '../components/ImageViewer.svelte';
  import UndoToast from '../components/UndoToast.svelte';
  import LinkSolution from '../components/LinkSolution.svelte';
  import { fresh } from '../lib/motion.svelte';
  import { dismiss, onWrote, say } from '../components/undoState.svelte';

  let path = $state(location.pathname);
  const match = $derived(matchRoute(path));
  const Page = $derived(match?.component);
  // A page whose feature is switched off says so instead of opening.
  const off = $derived(FEATURES.find((f) => f.name === ROUTES['/' + path.split('/')[1]] && !isOn(f.name)));

  // 'loading' draws nothing, so a set-up workspace never flashes the setup screen.
  let setup = $state<'loading' | 'needed' | 'done'>('loading');
  let overlay = $state<'palette' | 'quickadd' | null>(null);
  let quickAddDefaults = $state<IssueDefaults | null>(null);
  let refreshed = $state(0); // bumps when a command wrote, so the open view loads its rows again
  let palettePage = $state<string | undefined>();
  let paletteIssues = $state(false);
  let restoreOpen = $state(false);
  let cheat = $state(false); // CheatSheet opens itself on help.shortcuts; this only lets the root close it
  let createdIssue = $state<{ id: number; key: string } | null>(null);

  // Rows from every view select through `selectedIssueId`; the peek key goes through the selection store.
  const panelId = $derived(selectedIssueId.value ?? selection.peeked);
  // An id missing from the store (deleted, or outside the current view) falls through to the panel's not-found message.
  const pageKey = $derived(path.startsWith('/issue/') ? decodeURIComponent(path.split('/')[2] ?? '') : null);
  const pageIssueId = $derived(pageKey === null ? null : (issues.find((i) => i.key === pageKey)?.id ?? null));
  const panelKey = $derived(panelId === null ? null : (issues.find((i) => i.id === panelId)?.key ?? (createdIssue?.id === panelId ? createdIssue.key : `#${panelId}`)));

  // What commands see; getters keep it live, so the key engine never holds a stale copy.
  const ctx: Ctx = {
    get route() { return path; },
    get selection() { return selection.selected; },
    get focusedIssueId() { return selection.focused; },
    get openIssueId() { return panelId ?? pageIssueId; },
    get layout() { return selection.layout; },
    get api() { return api; },
    navigate,
  };
  setContext('cmdctx', () => ctx);

  // The layout the current route shows, so layout-scoped keys (board `l`, calendar `m`) know where they are. A view
  // that changes its own layout later says so itself through `selection.setLayout`.
  const BY_NAME: Record<string, Layout> = { list: 'list', table: 'table', board: 'board', gantt: 'timeline', timeline: 'timeline', calendar: 'calendar' };
  $effect(() => {
    const [a, b, c] = path.split('/').slice(1);
    let l: Layout | null = null;
    if (a === 'view') l = BY_NAME[(c ?? views.find((v) => String(v.id) === b)?.def.layout ?? 'list').toLowerCase()] ?? 'list';
    else if (path === '/projects/timeline') l = 'timeline';
    selection.setLayout(l);
  });

  // The panel follows the focused row, so j and k page through issues while it is open.
  $effect(() => {
    const f = selection.focused;
    if (f !== null && untrack(() => selectedIssueId.value) !== null) selectedIssueId.value = f;
  });

  const LABELS: Record<string, string> = { my: 'My issues', inbox: 'Inbox', pinned: 'Pinned', cycles: 'Cycles', dashboard: 'Dashboard', decisions: 'Decisions', events: 'Events', event: 'Events', projects: 'Projects', views: 'Views', settings: 'Settings', design: 'Design' };
  // A parent part that has a page of its own links to it (Projects, Views, Events, Cycles, Settings).
  const PAGES: Record<string, string> = { view: '/views', views: '/views', project: '/projects', projects: '/projects', event: '/events', events: '/events', cycles: '/cycles', settings: '/settings' };
  const crumb = $derived.by(() => {
    const [a, b] = path.split('/').slice(1);
    const parent = (label: string) => ({ label, href: PAGES[a] });
    if (!a) return [{ label: 'Pinned' }];
    if (a === 'view') return [parent('Views'), { label: views.find((v) => String(v.id) === b)?.name ?? b }];
    if (a === 'project') return [parent('Projects'), { label: projects.find((p) => p.slug === b)?.name ?? b }];
    if (a === 'issue') return [{ label: 'Issues' }, { label: b }];
    if (a === 'event') return [parent('Events')]; // the page heading names it
    const top = LABELS[a] ?? a;
    return b ? [parent(top), { label: b[0].toUpperCase() + b.slice(1).replace(/-/g, ' ') }] : [{ label: top }];
  });

  onMount(() => {
    initTheme(api);
    // A backend without workspace_status (mock before it lands) counts as set up, so the app stays usable.
    void Promise.resolve()
      .then(() => api.workspace.status())
      .then((s) => (setup = s.setup_done ? 'done' : 'needed'))
      .catch(() => (setup = 'done'));
  });

  $effect(() => (setup === 'done' ? attachKeys(() => ctx) : undefined));

  async function openQuickAdd(extra: IssueDefaults | null = null) {
    let fromView: IssueDefaults | null = null;
    if (viewContext.inView) {
      const filter = viewContext.filter;
      const route = path;
      try {
        const [statuses, projects, propertyDefs, defaultStatus] = await Promise.all([
          api.statuses.list(), api.projects.list(), api.properties.listDefs(), api.settings.get('default_status'),
        ]);
        if (route !== path || filter !== viewContext.filter) return;
        fromView = defaultsFromFilter(filter, {
          statuses, projects, propertyDefs, defaultStatusId: typeof defaultStatus === 'number' ? defaultStatus : null,
        });
      } catch {
        say({ label: 'Could not load view defaults', tone: 'error' });
        return;
      }
    }
    const merged = mergeIssueDefaults(fromView, extra);
    quickAddDefaults = Object.keys(merged).length ? merged : null;
    overlay = 'quickadd';
  }

  // Registry commands that open a root surface announce themselves as window events.
  $effect(() => {
    const on: [string, (e: Event) => void][] = [
      ['command:palette.open', (e) => ((palettePage = (e as CustomEvent<{ page?: string } | null>).detail?.page), (paletteIssues = false), (overlay = 'palette'))],
      ['command:nav.search', () => ((palettePage = undefined), (paletteIssues = true), (overlay = 'palette'))],
      ['command:issue.create', (e) => openQuickAdd((e as CustomEvent<{ defaults?: IssueDefaults } | null>).detail?.defaults ?? null)],
      ['command:view.refresh', () => {
        if (path.startsWith('/view/')) dispatchEvent(new Event('rows:changed'));
        else if (path === '/my') {
          dispatchEvent(new Event('rows:changed'));
          void refreshAll({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
        }
        else refreshed++;
      }],
      ['command:settings.restore', () => (restoreOpen = true)],
    ];
    on.forEach(([n, f]) => addEventListener(n, f));
    return () => on.forEach(([n, f]) => removeEventListener(n, f));
  });

  // The panel keeps showing the issue it had while it slides out.
  let shownKey = $state<string | null>(null);
  $effect(() => { if (panelKey !== null) shownKey = panelKey; });

  // Moving to another page leaves the panel behind.
  $effect(() => {
    path;
    selectedIssueId.value = null;
    createdIssue = null;
    dismiss(); // a message about the page left behind
  });

  // Deferred a microtask: every handler of the same Esc press must still see the issue open (move.escape checks
  // `openIssueId` right after issue.detail.close has run), or the second one would also act.
  const closePanel = () =>
    queueMicrotask(() => {
      selectedIssueId.value = null;
      // togglePeek only ever closes the peek of the focused row, so point focus at it first.
      if (selection.peeked !== null) {
        selection.focus(selection.peeked);
        selection.togglePeek();
      }
    });

  // The data under every store changed: load it again, show the start page, and say what happened.
  async function restored(folder: string) {
    restoreOpen = false;
    await refreshAll();
    refreshed++;
    navigate('/');
    await tick(); // moving pages clears the toast, so say it after
    say({ label: `Restored from ${folder}`, tone: 'success' });
  }

  async function created({ issue, mode }: { issue: { id?: number; key: string }; mode?: 'close' | 'open' | 'keep' }) {
    const key = issue.key;
    fresh.key = key; // its row flashes once when it appears
    if (mode !== 'keep') overlay = null;
    const inView = viewContext.inView;
    const activeDef = viewContext.viewDef;
    if (mode !== 'open') {
      if (typeof issue.id === 'number') createdIssue = { id: issue.id, key };
      await refreshAll(activeDef ?? undefined).catch(() => {});
      if (inView) {
        dispatchEvent(new Event('rows:changed'));
        if (mode !== 'keep' && typeof issue.id === 'number') {
          selection.focus(issue.id);
          selectedIssueId.value = issue.id;
        }
      }
    }
    // The write's own toast names the backend's step; wait for it, then say what was made.
    await onWrote();
    if (key) say({ label: `Created ${key}`, tone: 'success', action: { run: 'undo', text: 'Undo' } });
  }
</script>

<svelte:window onpopstate={() => (path = location.pathname)} />

{#if setup === 'needed'}
  <Setup ondone={() => (setup = 'done')} />
{:else if setup === 'done'}
  <Shell {crumb} route={path} panelOpen={panelKey !== null} onsearch={() => ((palettePage = undefined), (paletteIssues = false), (overlay = 'palette'))}>
    {#snippet sidebar()}<Sidebar route={path} />{/snippet}
    {#snippet panel()}
      {#if shownKey !== null}
        {#key shownKey}<IssueDetail params={{ key: shownKey }} mode="panel" onclose={closePanel} onopen={(k) => navigate(`/issue/${k}`)} />{/key}
      {/if}
    {/snippet}
    {#if off}
      <EmptyState title={`${off.label} is switched off`} hint="Turn it on in Settings, Features." />
    {:else if Page}
      {#key refreshed}<Page params={match?.params ?? {}} />{/key}
    {:else}
      <p>Not found: {path}</p>
    {/if}
  </Shell>

  <RestoreDialog bind:open={restoreOpen} onrestored={restored} />

  {#if overlay === 'palette'}
    <CommandPalette page={palettePage} issues={paletteIssues} onclose={() => overlay === 'palette' && (overlay = null)} oncreate={() => openQuickAdd()} />
  {:else if overlay === 'quickadd'}
    <QuickAdd defaults={quickAddDefaults} onclose={() => (overlay = null)} oncreated={created} />
  {/if}
  <CheatSheet bind:open={cheat} />
  <div class="toasts"><UndoToast /></div>
  <LinkSolution />
  <ImageViewer />
{/if}

<style>
  .toasts { position: fixed; z-index: var(--z-toast); left: 50%; bottom: calc(var(--row-h) + var(--s-8)); transform: translateX(-50%); }
</style>

<script lang="ts">
  import { onMount, setContext } from 'svelte';
  import { matchRoute, navigate } from './routes';
  import { api } from '../lib/api';
  import { attachKeys, type Ctx } from '../lib/commands';
  import { selection } from '../lib/selection.svelte';
  import { issues, projects, selectedIssueId, views } from '../lib/stores.svelte';
  import { applyTheme } from '../lib/theme.svelte';
  import CheatSheet from '../components/CheatSheet.svelte';
  import CommandPalette from '../components/CommandPalette.svelte';
  import IssueDetail from '../components/IssueDetail.svelte';
  import QuickAdd from '../components/QuickAdd.svelte';
  import Setup from '../components/Setup.svelte';
  import Shell from '../components/Shell.svelte';
  import Sidebar from '../components/Sidebar.svelte';
  import UndoToast from '../components/UndoToast.svelte';
  import { dismiss, onWrote, undoToast } from '../components/undoState.svelte';

  let path = $state(location.pathname);
  const match = $derived(matchRoute(path));
  const Page = $derived(match?.component);

  // 'loading' draws nothing, so a set-up workspace never flashes the setup screen.
  let setup = $state<'loading' | 'needed' | 'done'>('loading');
  let overlay = $state<'palette' | 'quickadd' | null>(null);
  let cheat = $state(false);

  // Rows from every view select through `selectedIssueId`; the peek key goes through the selection store.
  const panelId = $derived(selectedIssueId.value ?? selection.peeked);
  // An id missing from the store (deleted, or outside the current view) falls through to the panel's not-found message.
  const panelKey = $derived(panelId === null ? null : (issues.find((i) => i.id === panelId)?.key ?? `#${panelId}`));

  // What commands see; getters keep it live, so the key engine never holds a stale copy.
  const ctx: Ctx = {
    get route() { return path; },
    get selection() { return selection.selected; },
    get focusedIssueId() { return selection.focused; },
    get openIssueId() { return panelId; },
    get api() { return api; },
    navigate,
  };
  setContext('cmdctx', () => ctx);

  const LABELS: Record<string, string> = { my: 'My issues', inbox: 'Inbox', pinned: 'Pinned', cycles: 'Cycles', projects: 'Projects', views: 'Views', settings: 'Settings', design: 'Design' };
  const crumb = $derived.by(() => {
    const [a, b] = path.split('/').slice(1);
    if (!a) return ['Pinned'];
    if (a === 'view') return ['Views', views.find((v) => String(v.id) === b)?.name ?? b];
    if (a === 'project') return ['Projects', projects.find((p) => p.slug === b)?.name ?? b];
    if (a === 'issue') return ['Issues', b];
    return [LABELS[a] ?? a, ...(b ? [b[0].toUpperCase() + b.slice(1)] : [])];
  });

  onMount(() => {
    applyTheme();
    // A backend without workspace_status (mock before it lands) counts as set up, so the app stays usable.
    void Promise.resolve()
      .then(() => api.workspace.status())
      .then((s) => (setup = s.setup_done ? 'done' : 'needed'))
      .catch(() => (setup = 'done'));
  });

  $effect(() => (setup === 'done' ? attachKeys(() => ctx) : undefined));

  // Registry commands that open a root surface announce themselves as window events.
  $effect(() => {
    const on: [string, () => void][] = [
      ['command:palette.open', () => (overlay = 'palette')],
      ['command:nav.search', () => (overlay = 'palette')],
      ['command:issue.create', () => (overlay = 'quickadd')],
      ['command:help.shortcuts', () => (cheat = true)],
    ];
    on.forEach(([n, f]) => addEventListener(n, f));
    return () => on.forEach(([n, f]) => removeEventListener(n, f));
  });

  // Opening the panel adds a history entry, so Esc (move.escape goes back while an issue is open) closes it and keeps the page.
  let pushed = false;
  $effect(() => {
    if (panelId !== null && !pushed) history.pushState({ panel: true }, '');
    pushed = panelId !== null;
  });

  function closePanel() {
    selectedIssueId.value = null;
    // togglePeek only ever closes the peek of the focused row, so point focus at it first.
    if (selection.peeked !== null) {
      selection.focus(selection.peeked);
      selection.togglePeek();
    }
  }

  function onpop() {
    path = location.pathname;
    if (!history.state?.panel) closePanel();
  }

  // Props the panel and quick-add take from other screens' tasks; unknown props are ignored until those land.
  const detailProps = $derived({ params: { key: panelKey ?? '' }, mode: 'panel', onclose: closePanel, onopen: () => panelKey && navigate(`/issue/${panelKey}`) });

  async function created(i: { key?: string } | string) {
    const key = typeof i === 'string' ? i : i?.key;
    overlay = null;
    // The write's own toast names the backend's step; wait for it, then say what was made.
    await onWrote();
    if (key) undoToast.value = { label: `Created ${key}`, tone: 'info', action: { run: 'undo', text: 'Undo (cmd+Z)' } };
  }
</script>

<svelte:window onpopstate={onpop} />

{#if setup === 'needed'}
  <Setup ondone={() => (setup = 'done')} />
{:else if setup === 'done'}
  <Shell {crumb} panelOpen={panelKey !== null} onsearch={() => (overlay = 'palette')} onhelp={() => (cheat = true)}>
    {#snippet sidebar()}<Sidebar route={path} />{/snippet}
    {#snippet panel()}
      {#if panelKey !== null}
        {#key panelKey}<IssueDetail {...detailProps} />{/key}
      {/if}
    {/snippet}
    {#if Page}
      <Page params={match?.params ?? {}} />
    {:else}
      <p>Not found: {path}</p>
    {/if}
  </Shell>

  {#if overlay === 'palette'}
    <CommandPalette onclose={() => (overlay = null)} oncreate={() => (overlay = 'quickadd')} />
  {:else if overlay === 'quickadd'}
    <QuickAdd onclose={() => (overlay = null)} oncreated={created} />
  {/if}
  {#if cheat}<CheatSheet onclose={() => (cheat = false)} />{/if}
  <div class="toasts"><UndoToast /></div>
{/if}

<style>
  .toasts { position: fixed; z-index: var(--z-toast); left: 50%; bottom: calc(var(--row-h) + var(--s-8)); transform: translateX(-50%); }
</style>

<script lang="ts">
  import { flushSync } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Issue, IssuePatch, Project, ResolvedPin, Status } from '../lib/bindings';
  import { shortcutFor } from '../lib/commands/registry';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { dateOf, humanDate } from './detail/format';
  import Comments from './detail/Comments.svelte';
  import Description from './detail/Description.svelte';
  import History from './detail/History.svelte';
  import Properties from './detail/Properties.svelte';
  import Relations from './detail/Relations.svelte';
  import SubIssues from './detail/SubIssues.svelte';

  // One component for the full page (/issue/:key) and the side panel (mode 'panel', closed through `onclose`).
  let { params = {}, mode = 'page', onclose, onopen }: {
    params?: { key?: string };
    mode?: 'page' | 'panel';
    onclose?: () => void;
    onopen?: (key: string) => void;
  } = $props();
  const key = $derived(params.key ?? '');

  let issue = $state<Issue | null>(null);
  let status = $state<Status | null>(null);
  let project = $state<Project | null>(null);
  let pin = $state<ResolvedPin | null>(null);
  let siblings = $state<string[]>([]);
  let error = $state('');
  let missing = $state(false);
  // Bumped after every change so the panels reload their own lists.
  let rev = $state(0);
  let openRow = $state<string | null>(null);
  let editingDesc = $state(false);
  let editingTitle = $state(false);
  let titleDraft = $state('');
  let copied = $state(false);

  async function load() {
    try {
      const i = await api.issues.getByKey(key);
      const [s, p, pins] = await Promise.all([api.statuses.list(), api.projects.list(), api.pins.listResolved()]);
      status = s.find((x) => x.id === i.status_id) ?? null;
      project = p.find((x) => x.id === i.project_id) ?? null;
      pin = pins.find((x) => x.pin.kind === 'Issue' && x.pin.target === i.key) ?? null;
      issue = i;
      missing = false;
      rev++;
    } catch (e) {
      const err = e as { kind?: string; message?: string };
      if (!issue || err.kind === 'NotFound') missing = err.kind === 'NotFound';
      error = err.message ?? String(e);
    }
  }

  /** Runs a write, reloads the issue, and shows a refusal (such as a Conflict) instead of dropping it. */
  async function act(write: Promise<unknown>) {
    error = '';
    try {
      await write;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
    await load();
  }
  const save = (patch: IssuePatch) => act(api.issues.update(issue!.id, patch));

  $effect(() => {
    key;
    issue = null;
    error = '';
    openRow = null;
    editingDesc = editingTitle = false;
    load();
  });
  // Neighbours come from the default list order, loaded once per screen.
  $effect(() => {
    api.views
      .query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false })
      .then((l) => (siblings = l.map((i) => i.key)));
  });

  const open = (k: string) => (onopen ? onopen(k) : navigate(`/issue/${k}`));
  // Closing the full page never leaves the app: back when there is somewhere to go back to, else home.
  const close = () => (onclose ? onclose() : history.length > 1 ? history.back() : navigate('/'));
  const step = (d: number) => {
    const at = siblings.indexOf(key) + d;
    if (siblings.indexOf(key) >= 0 && siblings[at]) open(siblings[at]);
  };
  const togglePin = () => act(pin ? api.pins.unpin(pin.pin.id) : api.pins.pin('Issue', issue!.key, issue!.title));
  const toggleArchive = () => act(issue!.archived_at ? api.issues.restore(issue!.id) : api.issues.archive(issue!.id));
  async function copyKey() {
    await navigator.clipboard?.writeText(issue!.key);
    copied = true;
    setTimeout(() => (copied = false), 1200);
  }
  const startTitle = () => {
    titleDraft = issue!.title;
    editingTitle = true;
  };
  function commitTitle(e: Event) {
    e.preventDefault();
    if (!editingTitle) return;
    editingTitle = false;
    const t = titleDraft.trim();
    if (t && t !== issue!.title) save({ title: t });
  }
  const focusInput = (node: HTMLElement) => node.querySelector('input')?.focus();

  // Esc closes an open editor first. A popover closes itself on the same key press, so an editor that closed a
  // moment ago also counts as "that Esc was already used".
  let usedAt = 0;
  $effect(() => {
    if (openRow === null && !editingDesc && !editingTitle) usedAt = performance.now();
  });
  const onClose = () => {
    flushSync(); // the popover may have closed in this same key press; let the stamp effect see it
    if (openRow !== null || editingDesc || editingTitle) {
      openRow = null;
      editingDesc = editingTitle = false;
    } else if (performance.now() - usedAt > 60) close();
  };
  $effect(() => {
    const edit = (e: Event) => (openRow = (e as CustomEvent<string>).detail);
    const on: [string, () => void][] = [
      ['close', onClose], ['next', () => step(1)], ['prev', () => step(-1)],
      ['pin', togglePin], ['archive', toggleArchive], ['copyKey', copyKey],
    ];
    const editDesc = () => (editingDesc = true);
    addEventListener('command:issue.description.edit', editDesc);
    addEventListener('command:issue.detail.edit', edit);
    for (const [n, f] of on) addEventListener(`command:issue.detail.${n}`, f);
    return () => {
      removeEventListener('command:issue.description.edit', editDesc);
      removeEventListener('command:issue.detail.edit', edit);
      for (const [n, f] of on) removeEventListener(`command:issue.detail.${n}`, f);
    };
  });
  const tone = $derived({ Backlog: 1, Unstarted: 2, Started: 3, Completed: 4, Canceled: 1 }[status?.category ?? 'Backlog']);
</script>

{#snippet crumb()}
  {#if mode === 'panel'}
    <span class="crumb" title={project?.name ?? 'No project'}>
      <button type="button" class="key" aria-label="Copy issue key" onclick={copyKey}>{copied ? 'Copied' : issue!.key}</button></span>
  {:else}
    <span class="crumb">{project?.name ?? 'No project'} <span class="sep" aria-hidden="true">/</span>
      <button type="button" class="key" aria-label="Copy issue key" onclick={copyKey}>{copied ? 'Copied' : issue!.key}</button></span>
  {/if}
{/snippet}

{#if issue}
  <article class="detail" data-mode={mode}>
    <PageHeader title={crumb} icon={project?.icon ?? 'project'} color={project?.color ?? undefined}>
      {#snippet actions()}
        <IconButton icon="arrow-up" label="Previous issue" shortcut={shortcutFor('issue.detail.prev')} disabled={siblings.indexOf(key) < 1} onclick={() => step(-1)} />
        <IconButton icon="arrow-down" label="Next issue" shortcut={shortcutFor('issue.detail.next')} disabled={siblings.indexOf(key) < 0 || siblings.indexOf(key) >= siblings.length - 1} onclick={() => step(1)} />
        <IconButton icon="pin" label={pin ? 'Unpin issue' : 'Pin issue'} shortcut={shortcutFor('issue.detail.pin')} pressed={!!pin} onclick={togglePin} />
        <IconButton icon="archive" label={issue!.archived_at ? 'Restore issue' : 'Archive issue'} shortcut={shortcutFor('issue.detail.archive')} pressed={!!issue!.archived_at} onclick={toggleArchive} />
        <IconButton icon="close" label="Close issue" shortcut="escape" onclick={close} />
      {/snippet}
    </PageHeader>

    <div class="scroll">
      <div class="hero" style:--tone={`var(--band-${tone})`}>
        {#if status}<span class="st"><StatusGlyph category={status.category} size={20} />{status.name}</span>{/if}
        {#if editingTitle}
          <form onsubmit={commitTitle} onfocusout={commitTitle} use:focusInput>
            <TextInput label="Title" bind:value={titleDraft} />
          </form>
        {:else}
          <h2><button type="button" aria-label="Title" onclick={startTitle}>{issue.title}</button></h2>
        {/if}
        {#if issue.archived_at}<p class="arch">Archived {humanDate(dateOf(issue.archived_at))}</p>{/if}
      </div>
      {#if error}<p role="alert" class="error">{error}</p>{/if}
      <div class="cols">
        <main>
          <Description {issue} {save} bind:editing={editingDesc} />
          <SubIssues {issue} {act} {rev} {open} />
          <Relations {issue} {act} {rev} {open} />
          <Comments {issue} {act} {rev} />
          <History {issue} {rev} />
        </main>
        <aside aria-label="Details">
          <h3>Details</h3>
          <Properties {issue} {save} {act} bind:openRow />
          <p class="meta">Created {humanDate(dateOf(issue.created_at))} · Updated {humanDate(dateOf(issue.updated_at))}</p>
        </aside>
      </div>
    </div>
  </article>
{:else if missing}
  <EmptyState title="No such issue">
    {#snippet action()}<p role="alert" class="error">{error}</p><Button variant="primary" onclick={close}>Go back</Button>{/snippet}
  </EmptyState>
{:else if error}
  <p role="alert" class="error">{error}</p>
{:else}
  <div class="loading" aria-busy="true"><Skeleton width="40%" height="var(--s-9)" /><Skeleton /><Skeleton width="70%" /></div>
{/if}

<style>
  .detail { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--sheet); }
  .scroll { flex: 1; overflow-y: auto; }
  .hero { display: grid; gap: var(--s-4); padding: var(--s-8) var(--s-8) var(--s-7); background: var(--tone); color: var(--band-ink); border-bottom: 2px solid var(--stone); }
  .st { display: inline-flex; align-items: center; gap: var(--s-4); font-size: var(--fs-sm); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--band-count); }
  .crumb { font-size: var(--fs-lg); font-weight: 600; color: var(--muted); }
  .sep { margin: 0 var(--s-2); }
  .key { padding: 0; border: 0; background: transparent; font: inherit; color: var(--ink); font-weight: 700; }
  .key:hover { text-decoration: underline dotted; text-underline-offset: var(--s-2); }
  h2 { margin: 0; }
  h2 button { display: block; width: 100%; padding: 0; border: 0; background: transparent; text-align: left; font-size: var(--fs-display); font-weight: 800; letter-spacing: -0.03em; line-height: var(--lh-tight); }
  h2 button:hover { text-decoration: underline dotted; text-underline-offset: var(--s-2); }
  .arch { font-weight: 700; color: var(--signal); }
  .cols { display: grid; grid-template-columns: minmax(0, 1fr) calc(var(--panel-w) - var(--s-4)); }
  main { display: grid; align-content: start; gap: var(--s-8); padding: var(--s-7) var(--s-8) var(--s-10); min-width: 0; }
  aside { padding: var(--s-7); background: var(--card); border-left: 1px solid var(--border); position: relative; overflow: hidden; }
  aside h3 { font-size: var(--fs-xs); font-weight: 800; letter-spacing: 0.1em; text-transform: uppercase; color: var(--stone); padding-bottom: var(--s-3); border-bottom: 2px solid var(--stone); }
  .meta { margin-top: var(--s-7); font-size: var(--fs-sm); color: var(--muted); }
  .error { margin: var(--s-5) var(--s-8) 0; color: var(--signal); font-weight: 600; }
  .loading { display: grid; gap: var(--s-5); padding: var(--s-8); }
  [data-mode='panel'] .cols { grid-template-columns: minmax(0, 1fr); }
  [data-mode='panel'] aside { border-left: 0; border-top: 1px solid var(--border); padding: var(--s-5) var(--s-7); }
  [data-mode='panel'] .key, .crumb { white-space: nowrap; }
  .detail :global(.top) { gap: var(--s-4); padding-inline: var(--s-5); }
  .detail :global(.top h1) { flex: none; }
  .detail :global(.actions) { gap: var(--s-2); }
  [data-mode='panel'] .hero, [data-mode='panel'] main { padding-inline: var(--s-7); }
</style>

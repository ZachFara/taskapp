<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { InboxChoice, Issue, IssuePatch, Project } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Select from '../ui/Select.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import Description from './detail/Description.svelte';

  type Mode = 'delegate' | 'defer' | 'next' | null;
  // The Inbox as it was when the page opened, oldest first. A processed issue stays in the list (as `gone`) so
  // "12 of 74" keeps its meaning and Back and Skip walk the same order.
  let order = $state<Issue[]>([]);
  let gone = $state(new Set<number>());
  let at = $state(0);
  let loaded = $state(false);
  let error = $state('');
  let mode = $state<Mode>(null);
  let note = $state('');
  let until = $state('');
  let projectId = $state('');
  let projects = $state<Project[]>([]);
  let editingDescription = $state(false);
  let title = $state('');

  const current = $derived(order[at] as Issue | undefined);
  const left = $derived(order.length - gone.size);
  const iso = (days: number) => {
    const d = new Date();
    d.setDate(d.getDate() + days);
    return d.toLocaleDateString('sv');
  };
  const ahead = (from: number, step: number) => {
    for (let i = from + step; i >= 0 && i < order.length; i += step) if (!gone.has(order[i].id)) return i;
    return null;
  };

  async function load(keep: number | null = null) {
    try {
      order = await api.inbox.list();
      gone = new Set();
      const i = keep === null ? -1 : order.findIndex((x) => x.id === keep);
      at = Math.max(i, 0);
      error = '';
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
    loaded = true;
  }
  $effect(() => {
    title = current?.title ?? '';
    mode = null;
    editingDescription = false;
  });

  const move = (to: number | null) => to === null ? (at = order.length) : (at = to);
  const skip = () => current && move(ahead(at, 1));
  const back = () => ahead(at, -1) !== null && move(ahead(at, -1));

  async function run(choice: InboxChoice) {
    if (!current) return;
    const id = current.id;
    try {
      await api.inbox.process(id, choice);
      error = '';
      gone = new Set([...gone, id]);
      move(ahead(at, 1));
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
  const open = (m: Exclude<Mode, null>) => {
    if (!current) return;
    mode = mode === m ? null : m;
    if (m === 'defer' && !until) until = iso(1);
  };
  const confirm = (e: Event) => {
    e.preventDefault();
    if (mode === 'delegate') void run({ Delegate: { note } }).then(() => (note = ''));
    else if (mode === 'defer') void run({ Defer: { until } });
    else if (mode === 'next') void run({ Next: { project: projectId ? Number(projectId) : null } });
  };

  const save = async (patch: IssuePatch) => {
    if (!current) return;
    try {
      order[at] = await api.issues.update(current.id, patch);
      error = '';
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  };
  const saveTitle = () => {
    const t = title.trim();
    if (current && t && t !== current.title) void save({ title: t });
    else title = current?.title ?? '';
  };

  $effect(() => {
    const on: Record<string, () => void> = {
      'process.do': () => void run('Done'),
      'process.delegate': () => open('delegate'),
      'process.defer': () => open('defer'),
      'process.next': () => open('next'),
      'process.someday': () => void run('Someday'),
      'process.trash': () => void run('Trash'),
      'process.skip': skip,
      'process.back': back,
      // An undo (or any write elsewhere) changed the Inbox: read it again and stay on this issue.
      'view.refresh': () => void load(current?.id ?? null),
    };
    for (const [k, f] of Object.entries(on)) addEventListener(`command:${k}`, f);
    return () => Object.entries(on).forEach(([k, f]) => removeEventListener(`command:${k}`, f));
  });

  onMount(() => {
    void load();
    api.projects.list().then((p) => (projects = p.filter((x) => !x.archived_at)), () => {});
  });
</script>

<PageHeader title="Process Inbox" icon="icon:inbox">
  {#snippet actions()}
    {#if current}<span class="progress" aria-live="polite">{at + 1} of {order.length}</span>{/if}
  {/snippet}
</PageHeader>

<div class="body">
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if !loaded}
    <Skeleton />
  {:else if current}
    <article class="card" aria-label={current.key}>
      <p class="key">{current.key}</p>
      <TextInput label="Title" bind:value={title} onchange={saveTitle} />
      <Description issue={current} {save} bind:editing={editingDescription} />

      <div class="choices" role="group" aria-label="Choices">
        <Button variant="primary" icon="check" kbd="o" onclick={() => run('Done')}>Do it now</Button>
        <Button icon="user" kbd="y" aria-pressed={mode === 'delegate'} onclick={() => open('delegate')}>Delegate</Button>
        <Button icon="clock" kbd="z" aria-pressed={mode === 'defer'} onclick={() => open('defer')}>Defer</Button>
        <Button icon="arrow-right" kbd="n" aria-pressed={mode === 'next'} onclick={() => open('next')}>Next</Button>
        <Button icon="star" kbd="b" onclick={() => run('Someday')}>Someday</Button>
        <Button variant="danger" icon="trash" kbd="q" onclick={() => run('Trash')}>Trash</Button>
      </div>

      {#if mode}
        <form class="ask" onsubmit={confirm} onkeydown={(e) => e.key === 'Escape' && (mode = null)}>
          {#if mode === 'delegate'}
            <TextInput label="Waiting on" placeholder="Waiting on whom?" bind:value={note} />
            <Button type="submit" variant="primary" disabled={!note.trim()}>Delegate</Button>
          {:else if mode === 'defer'}
            <TextInput label="Hide until" type="date" bind:value={until} />
            <Button onclick={() => (until = iso(1))}>Tomorrow</Button>
            <Button onclick={() => (until = iso(7))}>Next week</Button>
            <Button onclick={() => (until = iso(30))}>Next month</Button>
            <Button type="submit" variant="primary" disabled={!until}>Defer</Button>
          {:else}
            <Select label="Project" placeholder="No project" bind:value={projectId} items={[{ value: '', label: 'No project' }, ...projects.map((p) => ({ value: String(p.id), label: p.name }))]} />
            <Button type="submit" variant="primary">Move to Next</Button>
          {/if}
          <Button onclick={() => (mode = null)}>Cancel</Button>
        </form>
      {/if}

      <div class="nav">
        <Button icon="chevron-left" kbd="shift+arrowleft" disabled={ahead(at, -1) === null} onclick={back}>Back</Button>
        <Button icon="chevron-right" kbd="shift+arrowright" onclick={skip}>Skip</Button>
      </div>
    </article>
  {:else if left === 0}
    <EmptyState title="Inbox zero" hint={order.length ? `You processed ${order.length} ${order.length === 1 ? 'issue' : 'issues'}. Nothing is waiting.` : 'Nothing is waiting in the Inbox.'}>
      {#snippet action()}<Button variant="primary" onclick={() => navigate('/')}>Done</Button>{/snippet}
    </EmptyState>
  {:else}
    <EmptyState title="End of the list" hint={`${left} ${left === 1 ? 'issue was' : 'issues were'} skipped.`}>
      {#snippet action()}<Button variant="primary" onclick={() => load()}>Go through them again</Button>{/snippet}
    </EmptyState>
  {/if}
</div>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); }
  .progress { color: var(--muted); font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
  .card { display: grid; gap: var(--s-5); padding: var(--s-6); border: 1px solid var(--border); border-radius: var(--r-sheet); background: var(--card); }
  .key { margin: 0; color: var(--muted); font-size: var(--fs-sm); font-weight: 600; }
  .choices, .ask, .nav { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-3); }
  .ask :global(.field) { flex: 1 1 220px; width: auto; }
  .nav { justify-content: space-between; border-top: 1px solid var(--border); padding-top: var(--s-5); }
  .error { color: var(--warning); margin: 0; }
</style>

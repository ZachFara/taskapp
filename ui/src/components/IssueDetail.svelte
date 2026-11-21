<script lang="ts">
  import { api } from '../lib/api';
  import type { Issue, IssuePatch } from '../lib/bindings';
  import Comments from './detail/Comments.svelte';
  import Description from './detail/Description.svelte';
  import Fields from './detail/Fields.svelte';
  import History from './detail/History.svelte';
  import Properties from './detail/Properties.svelte';
  import Relations from './detail/Relations.svelte';
  import SubIssues from './detail/SubIssues.svelte';

  let { params = {} }: { params?: { key?: string } } = $props();
  const key = $derived(params.key ?? '');

  let issue = $state<Issue | null>(null);
  let error = $state('');
  // Bumped after every change so the panels reload their own lists.
  let rev = $state(0);
  let tab = $state<'comments' | 'history'>('comments');

  async function load() {
    try {
      issue = await api.issues.getByKey(key);
      rev++;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
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
    load();
  });
</script>

{#if issue}
  <article class="detail">
    <header>
      <span class="key">{issue.key}</span>
      <input class="title" aria-label="Title" value={issue.title}
        onchange={(e) => save({ title: e.currentTarget.value })} />
    </header>
    {#if error}<p role="alert" class="error">{error}</p>{/if}
    <div class="cols">
      <main>
        <Description {issue} {save} />
        <Properties {issue} {act} />
        <SubIssues {issue} {act} {rev} />
        <Relations {issue} {act} {rev} />
        <nav>
          <button aria-pressed={tab === 'comments'} onclick={() => (tab = 'comments')}>Comments</button>
          <button aria-pressed={tab === 'history'} onclick={() => (tab = 'history')}>History</button>
        </nav>
        {#if tab === 'comments'}<Comments {issue} {act} {rev} />{:else}<History {issue} {rev} />{/if}
      </main>
      <aside><Fields {issue} {save} {act} /></aside>
    </div>
  </article>
{:else if error}
  <p role="alert" class="error">{error}</p>
{/if}

<style>
  .detail { padding: 12px 16px; }
  header { display: flex; gap: 8px; align-items: center; }
  .key { color: #888; font-size: 12px; }
  .title { flex: 1; font-size: 18px; font-weight: 600; border: 1px solid transparent; background: none; }
  .title:focus { border-color: #888; }
  .cols { display: grid; grid-template-columns: 1fr 260px; gap: 24px; margin-top: 12px; }
  .error { color: #c0392b; }
  nav { margin-top: 12px; }
  nav button[aria-pressed='true'] { font-weight: 600; }
</style>

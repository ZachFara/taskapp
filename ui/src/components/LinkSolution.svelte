<script lang="ts">
  import { api } from '../lib/api';
  import type { Issue } from '../lib/bindings';
  import { linkPrompt, setLinkPrompt } from '../lib/linkPrompt.svelte';
  import Button from '../ui/Button.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import IssuePicker from './detail/IssuePicker.svelte';

  // Offered after one issue is moved to a Completed status. It never blocks: nothing waits for it, it takes no focus,
  // and Skip (or Escape inside it) closes it.
  let issue = $state<Issue | null>(null);
  let url = $state('');
  let error = $state('');

  async function offer(e: Event) {
    const { ids, status } = (e as CustomEvent<{ ids: number[]; status: number }>).detail;
    if (!linkPrompt.on || ids.length !== 1) return;
    try {
      const [statuses, links] = await Promise.all([api.statuses.list(), api.links.list(ids[0])]);
      // Nothing to ask when the status is not a Completed one or the issue already says where its fix is.
      if (statuses.find((s) => s.id === status)?.category !== 'Completed' || links.some((l) => l.kind === 'solution')) return;
      issue = await api.issues.get(ids[0]);
      url = error = '';
    } catch {
      // The prompt is a convenience; a failed lookup just means no prompt.
    }
  }
  $effect(() => {
    addEventListener('issues:status-set', offer);
    return () => removeEventListener('issues:status-set', offer);
  });

  async function add(link: string, title: string | null) {
    if (!issue) return;
    try {
      await api.links.add(issue.id, link, title, 'solution');
      issue = null;
      dispatchEvent(new Event('command:view.refresh'));
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
  const skip = () => (issue = null);
  function never() {
    setLinkPrompt(false);
    skip();
  }
</script>

{#if issue}
  <section class="prompt" aria-label="Link the solution" onkeydown={(e) => e.key === 'Escape' && skip()}>
    <h3>Link the solution to {issue.key}</h3>
    <form onsubmit={(e) => { e.preventDefault(); if (url.trim()) void add(url.trim(), null); }}>
      <TextInput label="Solution URL" placeholder="Paste a PR, commit or doc URL" bind:value={url} />
      <Button type="submit" variant="primary" disabled={!url.trim()}>Add link</Button>
    </form>
    <IssuePicker selfId={issue.id} label="Or find an issue" pick={(i) => void add(`taskapp://issue/${i.key}`, i.title)} />
    {#if error}<p role="alert" class="error">{error}</p>{/if}
    <div class="end">
      <Button variant="quiet" onclick={never}>Don't ask again</Button>
      <Button variant="quiet" onclick={skip}>Skip</Button>
    </div>
  </section>
{/if}

<style>
  .prompt { position: fixed; z-index: var(--z-toast); right: var(--s-8); bottom: calc(var(--row-h) + var(--s-8)); width: 360px; display: grid; gap: var(--s-4); padding: var(--s-6); background: var(--sheet); color: var(--ink); border: 1px solid var(--border); border-radius: var(--r-pop); box-shadow: var(--shadow-float); }
  h3 { font-size: var(--fs-sm); font-weight: 600; }
  form { display: grid; grid-template-columns: 1fr auto; gap: var(--s-4); }
  .end { display: flex; justify-content: flex-end; gap: var(--s-2); }
  .error { color: var(--signal); }
</style>

<script lang="ts">
  import { api } from '../../lib/api';
  import type { Comment, Issue } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import { renderMarkdown } from './markdown';

  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  let comments = $state<Comment[]>([]);
  let draft = $state('');
  let collapsed = $state(false);

  $effect(() => {
    rev;
    api.comments.list(issue.id).then((c) => (comments = c));
  });
</script>

<section aria-label="Comments">
  <GroupHeader name="Comments" count={comments.length} category="Started" {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  {#if !collapsed}
    {#each comments as c (c.id)}
      <article class="comment">
        <time datetime={c.created_at}>{c.created_at.slice(0, 16).replace('T', ' ')}</time>
        <div class="md">{@html renderMarkdown(c.body)}</div>
      </article>
    {:else}
      <EmptyState title="No comments yet" hint="Notes, decisions and links go here." />
    {/each}
    <div class="new">
      <TextArea label="New comment" rows={3} placeholder="Leave a comment (markdown)" bind:value={draft} />
      <Button variant="primary" disabled={!draft.trim()} onclick={() => { act(api.comments.addComment(issue.id, draft)); draft = ''; }}>Comment</Button>
    </div>
  {/if}
</section>

<style>
  .comment { padding: var(--s-5) var(--s-5); border-bottom: 1px dotted var(--control); }
  time { display: block; font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); }
  .md :global(p) { margin: var(--s-2) 0; }
  .new { display: grid; justify-items: start; gap: var(--s-4); padding: var(--s-5); }
</style>

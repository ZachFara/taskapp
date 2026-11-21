<script lang="ts">
  import { api } from '../../lib/api';
  import type { Comment, Issue } from '../../lib/bindings';
  import { renderMarkdown } from './markdown';

  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  let comments = $state<Comment[]>([]);
  let draft = $state('');

  $effect(() => {
    rev;
    api.comments.list(issue.id).then((c) => (comments = c));
  });
</script>

<section aria-label="Comments">
  {#each comments as c (c.id)}
    <div class="comment">{@html renderMarkdown(c.body)}</div>
  {/each}
  <textarea aria-label="New comment" rows="3" bind:value={draft}></textarea>
  <button disabled={!draft.trim()} onclick={() => { act(api.comments.addComment(issue.id, draft)); draft = ''; }}>Comment</button>
</section>

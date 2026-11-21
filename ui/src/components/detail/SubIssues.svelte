<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue } from '../../lib/bindings';
  import IssuePicker from './IssuePicker.svelte';

  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  let children = $state<Issue[]>([]);

  $effect(() => {
    rev;
    api.issues.children(issue.id).then((c) => (children = c));
  });
  const attach = (child: Issue) => child.id !== issue.id && act(api.issues.update(child.id, { parent_id: issue.id }));
</script>

<section aria-label="Sub-issues">
  <h3>Sub-issues</h3>
  <ul>
    {#each children as c (c.id)}<li>{c.key} {c.title}</li>{/each}
  </ul>
  <IssuePicker selfId={issue.id} label="Add sub-issue" pick={attach} />
  <input aria-label="New sub-issue title" placeholder="New sub-issue"
    onchange={(e) => {
      const title = e.currentTarget.value.trim();
      e.currentTarget.value = '';
      if (title) act(api.issues.create({ title, parent_id: issue.id }));
    }} />
</section>

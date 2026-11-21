<script lang="ts">
  import { api } from '../../lib/api';
  import type { Change, Issue } from '../../lib/bindings';

  let { issue, rev }: { issue: Issue; rev: number } = $props();
  let changes = $state<Change[]>([]);

  $effect(() => {
    rev;
    api.history.listFor('issue', String(issue.id)).then((c) => (changes = c));
  });
</script>

<section aria-label="History">
  <ul>
    {#each [...changes].reverse() as c (c.id)}
      <li>{c.at} {c.field}: {JSON.stringify(c.old_value)} to {JSON.stringify(c.new_value)}</li>
    {:else}
      <li>No changes yet</li>
    {/each}
  </ul>
</section>

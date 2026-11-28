<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue } from '../../lib/bindings';
  import Collapse from '../../ui/Collapse.svelte';
  import SectionHeading from './SectionHeading.svelte';

  // The three open issues most like this one's title. Marking one a duplicate makes this issue the copy.
  let { issue, act, rev, open }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number; open: (key: string) => void } = $props();
  let similar = $state<Issue[]>([]);
  let collapsed = $state(false);

  $effect(() => {
    rev;
    api.search.similar(issue.title, issue.project_id ?? null, 4).then(
      (found) => (similar = found.filter((i) => i.id !== issue.id).slice(0, 3)),
      () => (similar = []),
    );
  });
</script>

{#if similar.length}
  <section aria-label="Similar issues">
    <SectionHeading name="Similar issues" count={similar.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
    <Collapse open={!collapsed}>
      <ul>
        {#each similar as s (s.id)}
          <li data-similar>
            <button type="button" class="go" onclick={() => open(s.key)}><b>{s.key}</b> {s.title}</button>
            <button type="button" class="dupe" aria-label="Mark {issue.key} a duplicate of {s.key}" onclick={() => act(api.issues.markDuplicate(issue.id, s.id))}>Mark duplicate</button>
          </li>
        {/each}
      </ul>
    </Collapse>
  </section>
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; height: var(--row-h); padding-right: var(--s-4); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; color: var(--ink); }
  .go:hover { background: var(--active); }
  .dupe { border: 0; background: transparent; color: var(--muted); font-size: var(--fs-sm); padding: 0 var(--s-3); cursor: pointer; }
  .dupe:hover, .dupe:focus-visible { color: var(--ink); }
</style>

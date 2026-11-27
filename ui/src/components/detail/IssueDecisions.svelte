<script lang="ts">
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Decision, Issue } from '../../lib/bindings';
  import SectionHeading from './SectionHeading.svelte';

  // The open decisions that name this issue; the section is absent when there are none.
  let { issue, rev }: { issue: Issue; rev: number } = $props();
  let mine = $state<Decision[]>([]);
  let collapsed = $state(false);

  $effect(() => {
    rev;
    api.decisions.list('open').then((l) => (mine = l.filter((d) => d.issue_ids.includes(issue.id))), () => (mine = []));
  });
</script>

{#if mine.length}
  <section aria-label="Decisions">
    <SectionHeading name="Decisions" count={mine.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
    {#if !collapsed}
      <ul>
        {#each mine as d (d.id)}
          <li><button type="button" class="go" onclick={() => navigate('/decisions')}>{#if d.mode === 'blocking'}<em>Blocking</em>{/if}{d.question}</button></li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; height: var(--row-h); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  em { font-style: normal; font-size: var(--fs-xs); font-weight: 700; color: var(--c-red); text-transform: uppercase; letter-spacing: 0.06em; }
</style>

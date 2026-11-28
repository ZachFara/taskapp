<script lang="ts">
  import { navigate } from '../../app/routes';
  import type { CardProps } from './cards';

  let { stuck: rows }: CardProps = $props();
</script>

{#if rows.length === 0}
  <p class="quiet">No project is stuck. A project is stuck when it is in progress, is not an Area or Collection, and has no open next action.</p>
{:else}
  <p class="count">{rows.length} {rows.length === 1 ? 'project has' : 'projects have'} no next action</p>
  <ul>
    {#each rows as r (r.project.id)}
      <li><a href={`/project/${r.project.slug}`} onclick={(e) => (e.preventDefault(), navigate(`/project/${r.project.slug}`))}>{r.project.name}</a></li>
    {/each}
  </ul>
{/if}

<style>
  .quiet, .count { margin: 0 0 var(--s-4); color: var(--muted); font-size: var(--fs-sm); }
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-2); font-size: var(--fs-sm); }
  a { color: inherit; text-decoration: none; }
  a:hover { text-decoration: underline; }
</style>

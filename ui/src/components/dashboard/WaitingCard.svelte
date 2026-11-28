<script lang="ts">
  import { navigate } from '../../app/routes';
  import type { CardProps } from './cards';

  // The issues whose follow-up day has come (the page filters them).
  let { waiting: rows }: CardProps = $props();
</script>

{#if rows.length === 0}
  <p class="quiet">No follow-up is due. An issue you are waiting on shows here once its follow-up day comes.</p>
{:else}
  <p class="count">{rows.length} {rows.length === 1 ? 'follow-up is' : 'follow-ups are'} due</p>
  <ul>
    {#each rows as i (i.id)}
      <li><a href={`/issue/${i.key}`} onclick={(e) => (e.preventDefault(), navigate(`/issue/${i.key}`))}>{i.key} {i.title}</a>{#if i.waiting_on}<span class="who"> waiting on {i.waiting_on}</span>{/if}</li>
    {/each}
  </ul>
{/if}

<style>
  .quiet, .count { margin: 0 0 var(--s-4); color: var(--muted); font-size: var(--fs-sm); }
  ul { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-2); font-size: var(--fs-sm); }
  a { color: inherit; text-decoration: none; }
  a:hover { text-decoration: underline; }
  .who { color: var(--muted); }
</style>

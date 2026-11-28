<script lang="ts">
  import { navigate } from '../../app/routes';
  import type { CardProps } from './cards';

  let { lastReview, today }: CardProps = $props();
  // Whole days between two YYYY-MM-DD workspace days (both read as UTC midnight, so no DST drift).
  const days = $derived(lastReview && today ? Math.round((Date.parse(`${today}T00:00:00Z`) - Date.parse(`${lastReview}T00:00:00Z`)) / 864e5) : null);
  const go = (e: MouseEvent) => (e.preventDefault(), navigate('/review'));
</script>

{#if days === null}
  <p class="count">No weekly review yet</p>
{:else}
  <p class="count">{days} {days === 1 ? 'day' : 'days'} since your weekly review</p>
  <p class="quiet">Last finished {lastReview}.</p>
{/if}
<p class="quiet"><a href="/review" onclick={go}>Start the weekly review</a></p>

<style>
  .quiet, .count { margin: 0 0 var(--s-3); color: var(--muted); font-size: var(--fs-sm); }
  .count { color: inherit; font-size: var(--fs-lg); font-weight: 600; }
  a { color: inherit; text-decoration: underline; }
</style>

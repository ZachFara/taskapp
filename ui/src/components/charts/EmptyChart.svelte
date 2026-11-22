<script lang="ts">
  import Contour from '../../ui/Contour.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';

  // The two states every chart shares: still loading, or nothing to draw yet.
  let { loading = false, height = 120 }: { loading?: boolean; height?: number } = $props();
</script>

{#if loading}
  <div aria-busy="true" style:height={`${height}px`}><Skeleton height="100%" /></div>
{:else}
  <div class="quiet" style:height={`${height}px`}>
    <span class="art" aria-hidden="true"><Contour rings={4} size={64} /></span>
    <span>Progress appears once issues move</span>
  </div>
{/if}

<style>
  .quiet { display: flex; align-items: center; gap: var(--s-5); color: var(--muted); font-size: var(--fs-md); }
  .art { color: var(--stone); opacity: 0.35; display: inline-flex; }
</style>

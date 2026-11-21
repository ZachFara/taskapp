<script lang="ts">
  import { matchRoute } from './routes';

  let path = $state(location.pathname);
  const match = $derived(matchRoute(path));
  const Page = $derived(match?.component);
</script>

<svelte:window onpopstate={() => (path = location.pathname)} />

{#if Page}
  <Page />
{:else}
  <p>Not found: {path}</p>
{/if}

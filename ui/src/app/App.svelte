<script lang="ts">
  import { matchRoute } from './routes';
  import { refreshAll } from '../lib/stores.svelte';

  refreshAll();

  let path = $state(location.pathname);
  const match = $derived(matchRoute(path));
  const Page = $derived(match?.component);
</script>

<svelte:window onpopstate={() => (path = location.pathname)} />

{#if Page}
  <Page params={match?.params ?? {}} />
{:else}
  <p>Not found: {path}</p>
{/if}

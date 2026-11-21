<script lang="ts">
  import { matchRoute } from './routes';
  import { issues, selectedIssueId } from '../lib/stores.svelte';
  import IssueDetail from '../components/IssueDetail.svelte';
  import Shell from '../components/Shell.svelte';
  import Sidebar from '../components/Sidebar.svelte';

  let path = $state(location.pathname);
  const match = $derived(matchRoute(path));
  const Page = $derived(match?.component);
  // An id missing from the store (deleted, or outside the current view) falls through to the panel's not-found message.
  const panelKey = $derived(
    selectedIssueId.value === null ? null : (issues.find((i) => i.id === selectedIssueId.value)?.key ?? `#${selectedIssueId.value}`),
  );

  $effect(() => {
    path;
    selectedIssueId.value = null;
  });

  // An open palette or quick-add owns Esc; the panel only closes when none is showing.
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !document.querySelector('[role=dialog]')) selectedIssueId.value = null;
  }
</script>

<svelte:window onpopstate={() => (path = location.pathname)} {onkeydown} />

<Shell>
  {#snippet sidebar()}<Sidebar />{/snippet}
  {#snippet panel()}
    {#if panelKey !== null}
      {#key panelKey}<IssueDetail params={{ key: panelKey }} />{/key}
    {/if}
  {/snippet}
  {#if Page}
    <Page params={match?.params ?? {}} />
  {:else}
    <p>Not found: {path}</p>
  {/if}
</Shell>

<script lang="ts">
  import CommandButton from './CommandButton.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
</script>

<PageHeader title="Inbox" icon="icon:inbox" />
<EmptyState title="Inbox arrives in a later release" hint="Triage for captured items and mentions will land here. Nothing is waiting for you yet.">
  {#snippet action()}<CommandButton command="issue.create" variant="primary" />{/snippet}
</EmptyState>

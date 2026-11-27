<script lang="ts">
  import Dialog from '../ui/Dialog.svelte';
  import RestoreForm from './RestoreForm.svelte';

  let { open = $bindable(false), onrestored }: { open?: boolean; onrestored: (folder: string) => void | Promise<void> } = $props();
</script>

<Dialog bind:open title="Restore from export" size="md">
  <p class="warn">
    This replaces everything in this workspace (issues, projects, views, statuses, labels and settings) with the export's.
    A backup of the current database is saved first. Sign-ins to Google and Reminders stay as they are.
  </p>
  <RestoreForm {onrestored} />
</Dialog>

<style>
  .warn { color: var(--ink); line-height: var(--lh-body); margin-bottom: var(--s-6); }
</style>

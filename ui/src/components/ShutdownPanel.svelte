<script lang="ts">
  import { shutdown } from '../lib/shutdown.svelte';
  import CommandButton from './CommandButton.svelte';
  import Checkbox from '../ui/Checkbox.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import TextInput from '../ui/TextInput.svelte';

  // The end-of-day checklist. Its actions are registry commands (shutdown.*), the buttons here run the same ones.
  const picked = (id: number) => shutdown.picked.includes(id);
  const toggle = (id: number, on: boolean) => {
    shutdown.picked = on ? [...shutdown.picked, id] : shutdown.picked.filter((x) => x !== id);
  };
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;
</script>

<Dialog bind:open={shutdown.open} title="Shutdown" size="md">
  {#if shutdown.state}
    {@const s = shutdown.state}
    <section aria-label="Inbox">
      <h3>Inbox reviewed</h3>
      <p>{s.inbox_count === 0 ? 'The Inbox is empty.' : `${plural(s.inbox_count, 'issue')} left in the Inbox.`}</p>
    </section>

    <section aria-label="Tomorrow">
      <h3>Tomorrow's deep work (up to 3)</h3>
      {#if s.deep.length}
        <ul class="picks">
          {#each s.deep as issue (issue.id)}
            <li>
              <Checkbox
                label={issue.title}
                checked={picked(issue.id)}
                disabled={!picked(issue.id) && shutdown.picked.length >= 3}
                onchange={(on) => toggle(issue.id, on)}
              />
              <span>{issue.key} {issue.title}</span>
            </li>
          {/each}
        </ul>
        <CommandButton command="shutdown.saveTomorrow" />
      {:else}
        <p>No open issue is marked deep. Set a depth on an issue to pick it here.</p>
      {/if}
    </section>

    <section aria-label="Loose ends">
      <h3>Loose ends</h3>
      <div class="capture">
        <TextInput bind:value={shutdown.loose} label="Loose end" placeholder="Quick add, like Call the bank fri" />
        <CommandButton command="shutdown.capture" />
      </div>
    </section>

    <footer>
      {#if s.done_today}
        <p role="status">Shutdown complete. {plural(s.streak, 'day')} in a row.</p>
      {:else}
        <p>{s.streak > 0 ? `${plural(s.streak, 'day')} in a row so far.` : 'Finish today to start a streak.'}</p>
        <CommandButton command="shutdown.finish" variant="primary" />
      {/if}
    </footer>
  {/if}
</Dialog>

<style>
  h3 { margin: var(--s-5) 0 var(--s-2); font-size: var(--fs-lg); }
  p { margin: 0 0 var(--s-3); line-height: var(--lh-body); }
  .picks { list-style: none; margin: 0 0 var(--s-3); padding: 0; display: grid; gap: var(--s-2); }
  .picks li { display: flex; align-items: center; gap: var(--s-3); }
  .capture { display: flex; gap: var(--s-3); align-items: flex-end; }
  footer { margin-top: var(--s-6); display: flex; align-items: center; justify-content: space-between; gap: var(--s-3); flex-wrap: wrap; }
  footer p { margin: 0; }
</style>

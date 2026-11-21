<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let lists = $state<[string, string][]>([]);
  let current = $state('');
  let error = $state('');

  onMount(async () => {
    current = ((await api.settings.get('reminders_list_id')) as string | null) ?? '';
  });

  // Reading the lists is what makes macOS ask for Reminders access, so this is the authorize step.
  async function authorize() {
    error = '';
    try {
      lists = await api.reminders.listLists();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function pick(id: string) {
    current = id;
    await api.settings.set('reminders_list_id', id);
  }
</script>

<h2>Reminders</h2>
<p>Tasks you add to the inbox list in Reminders are imported into the app.</p>
<button onclick={authorize}>Authorize</button>
{#if lists.length}
  <label>
    Inbox list
    <select aria-label="Inbox list" value={current} onchange={(e) => void pick(e.currentTarget.value)}>
      <option value="">Choose a list</option>
      {#each lists as [id, title] (id)}<option value={id}>{title}</option>{/each}
    </select>
  </label>
{/if}
{#if error}<p role="alert">{error}</p>{/if}

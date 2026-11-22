<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import Select from '../../ui/Select.svelte';

  let lists = $state<[string, string][]>([]);
  let current = $state('');
  let error = $state('');
  // The value the settings store holds; the effect writes only when the owner picks a different list.
  let stored = '';

  const items = $derived(lists.map(([id, title]) => ({ value: id, label: title })));
  const connected = $derived(lists.length > 0);

  onMount(async () => {
    stored = current = ((await api.settings.get('reminders_list_id')) as string | null) ?? '';
  });

  $effect(() => {
    const id = current;
    if (id === stored) return;
    stored = id;
    void api.settings.set('reminders_list_id', id);
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
</script>

<SectionHead lede="Tasks you add to the inbox list in Reminders are imported into the app." />

<div class="st-card">
  <span class="st-status" data-state={connected ? 'ok' : undefined} data-status>{connected ? 'Connected' : 'Not set up'}</span>
  <p class="st-hint">
    {connected ? 'Choose the list the app reads from.' : 'Authorize once so macOS lets the app read your Reminders lists.'}
  </p>
  <div class="st-actions">
    <Button variant="primary" onclick={authorize}>Authorize</Button>
  </div>
  {#if connected}
    <div class="st-field">
      <span>Inbox list</span>
      <Select label="Inbox list" items={items} bind:value={current} placeholder="Choose a list" />
    </div>
  {/if}
  {#if error}<p role="alert" class="st-error">{error}</p>{/if}
</div>

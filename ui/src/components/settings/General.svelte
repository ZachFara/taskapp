<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let prefix = $state('');
  let error = $state('');
  let saved = $state(false);

  onMount(async () => {
    prefix = ((await api.settings.get('issue_prefix')) as string | null) ?? '';
  });

  async function save() {
    if (!/^[A-Z][A-Z0-9]*$/.test(prefix)) {
      error = 'Use uppercase letters and digits, starting with a letter.';
      return;
    }
    error = '';
    try {
      await api.settings.set('issue_prefix', prefix);
      saved = true;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>General</h2>
<form onsubmit={(e) => { e.preventDefault(); void save(); }}>
  <label>
    Issue prefix
    <input aria-label="Issue prefix" bind:value={prefix} oninput={() => (saved = false)} />
  </label>
  <p>New issues get keys such as {prefix || 'ZF'}-1. Existing keys do not change.</p>
  {#if error}<p role="alert">{error}</p>{/if}
  <button type="submit">Save</button>
  {#if saved}<span>Saved</span>{/if}
</form>

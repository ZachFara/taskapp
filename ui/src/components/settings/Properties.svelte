<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { projects, propertyDefs, refreshAll } from '../../lib/stores.svelte';
  import type { PropertyKind, SelectOption } from '../../lib/bindings';

  const kinds: PropertyKind[] = ['Text', 'Number', 'Date', 'Select', 'MultiSelect', 'Checkbox', 'Url', 'NoteLink'];
  const keyPattern = /^[a-z][a-z0-9_]{0,39}$/;

  let selected = $state('');
  let error = $state('');
  let confirming = $state(false);
  let newName = $state('');
  let newKey = $state('');
  let newKind = $state<PropertyKind>('Text');
  let newScope = $state('');
  let renameTo = $state('');
  let changeTo = $state<PropertyKind>('Text');
  let scopeTo = $state('');

  const current = $derived(propertyDefs.find((d) => d.key === selected));

  // The scope control shows the selected property's real scope, also after a refused change.
  $effect(() => {
    scopeTo = current?.project_id == null ? '' : String(current.project_id);
  });

  onMount(async () => {
    await refreshAll();
    selected = propertyDefs[0]?.key ?? '';
  });

  // Every write goes through here so a refused write shows its message and the lists stay in step.
  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  function add() {
    if (!keyPattern.test(newKey)) {
      error = 'Keys start with a letter and use lowercase letters, digits and underscores.';
      return;
    }
    const project_id = newScope === '' ? null : Number(newScope);
    void write(async () => {
      await api.properties.createDef({
        key: newKey, name: newName.trim() || newKey, kind: newKind, options: [], project_id,
        position: propertyDefs.length,
      });
      newName = newKey = '';
    });
  }

  function rename() {
    void write(() => api.properties.renameDef(selected, renameTo.trim()));
  }

  function changeKind() {
    const options: SelectOption[] = changeTo === 'Select' || changeTo === 'MultiSelect' ? current?.options ?? [] : [];
    void write(() => api.properties.changeKind(selected, changeTo, options));
  }

  function setScope() {
    void write(() => api.properties.setScope(selected, scopeTo === '' ? null : Number(scopeTo)));
  }

  function move(by: -1 | 1) {
    const keys = propertyDefs.map((d) => d.key);
    const at = keys.indexOf(selected);
    if (at + by < 0 || at + by >= keys.length) return;
    [keys[at], keys[at + by]] = [keys[at + by], keys[at]];
    void write(() => api.properties.reorder(keys));
  }

  function remove() {
    void write(async () => {
      await api.properties.deleteDef(selected);
      confirming = false;
      selected = propertyDefs[0]?.key ?? '';
    });
  }
</script>

<h2>Properties</h2>
<ul>
  {#each propertyDefs as d (d.key)}
    <li>{d.name} <code>{d.key}</code> {d.kind}{d.project_id ? ` (project ${d.project_id})` : ''}</li>
  {/each}
</ul>

<form onsubmit={(e) => { e.preventDefault(); add(); }}>
  <h3>Add property</h3>
  <input aria-label="New property name" placeholder="Name" bind:value={newName} />
  <input aria-label="New property key" placeholder="key" bind:value={newKey} />
  <select aria-label="New property kind" bind:value={newKind}>
    {#each kinds as k (k)}<option value={k}>{k}</option>{/each}
  </select>
  <select aria-label="New property scope" bind:value={newScope}>
    <option value="">All projects</option>
    {#each projects as p (p.id)}<option value={String(p.id)}>{p.name}</option>{/each}
  </select>
  <button type="submit">Add property</button>
</form>

<div>
  <h3>Edit property</h3>
  <select aria-label="Property" bind:value={selected} onchange={() => (confirming = false)}>
    {#each propertyDefs as d (d.key)}<option value={d.key}>{d.name}</option>{/each}
  </select>
  <input aria-label="Rename to" bind:value={renameTo} />
  <button onclick={rename}>Rename</button>
  <select aria-label="New kind" bind:value={changeTo}>
    {#each kinds as k (k)}<option value={k}>{k}</option>{/each}
  </select>
  <button onclick={changeKind}>Change kind</button>
  <select aria-label="Scope" bind:value={scopeTo}>
    <option value="">All projects</option>
    {#each projects as p (p.id)}<option value={String(p.id)}>{p.name}</option>{/each}
  </select>
  <button onclick={setScope}>Set scope</button>
  <button onclick={() => move(-1)}>Move up</button>
  <button onclick={() => move(1)}>Move down</button>
  {#if confirming}
    <button onclick={remove}>Confirm delete</button>
    <button onclick={() => (confirming = false)}>Cancel</button>
  {:else}
    <button onclick={() => (confirming = true)} disabled={!selected}>Delete</button>
  {/if}
</div>

{#if error}<p role="alert">{error}</p>{/if}

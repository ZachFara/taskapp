<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { RelationKindDef } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Pill from '../../ui/Pill.svelte';
  import Switch from '../../ui/Switch.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  let kinds = $state<RelationKindDef[]>([]);
  let draft = $state({ name: '', forward: '', reverse: '', symmetric: false });
  let error = $state('');
  let doomed = $state<RelationKindDef | null>(null);

  const load = async () => (kinds = await api.relation.kindsList());
  onMount(load);

  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
    await load();
  }

  const add = () => write(async () => {
    await api.relation.kindsCreate(draft.name, draft.forward, draft.reverse, draft.symmetric);
    draft = { name: '', forward: '', reverse: '', symmetric: false };
  });
  const change = (k: RelationKindDef, patch: Partial<RelationKindDef>) => write(() => api.relation.kindsUpdate({ ...k, ...patch }));
  const remove = () => write(async () => {
    await api.relation.kindsDelete(doomed!.id);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="How issues can relate. Each kind reads one way from the first issue and the other way from the second, like blocks and blocked by." />

  <ul class="es-list">
    {#each kinds as k (k.id)}
      <li class="es-row">
        {#if k.built_in}
          <span class="es-name"><b>{k.name}</b></span>
          <span class="es-meta">{k.forward}{k.symmetric ? '' : ` / ${k.reverse}`}</span>
          <Pill>Built in</Pill>
        {:else}
          <div class="es-name"><TextInput label={`Name of ${k.name}`} value={k.name} onchange={(v) => v.trim() && v !== k.name && change(k, { name: v })} /></div>
          <div class="es-name"><TextInput label={`Forward label of ${k.name}`} value={k.forward} onchange={(v) => v.trim() && v !== k.forward && change(k, { forward: v })} /></div>
          {#if !k.symmetric}
            <div class="es-name"><TextInput label={`Reverse label of ${k.name}`} value={k.reverse} onchange={(v) => v.trim() && v !== k.reverse && change(k, { reverse: v })} /></div>
          {/if}
          <Switch label={`Same both ways: ${k.name}`} checked={k.symmetric} onclick={() => change(k, { symmetric: !k.symmetric })} />
          <div class="es-tools"><IconButton icon="trash" label={`Delete ${k.name}`} onclick={() => (doomed = k)} /></div>
        {/if}
      </li>
    {/each}
  </ul>

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <div class="es-name"><TextInput label="Kind name" placeholder="New kind, e.g. Mentors" bind:value={draft.name} /></div>
    <div class="es-name"><TextInput label="Forward label" placeholder="reads from the first issue" bind:value={draft.forward} /></div>
    {#if !draft.symmetric}
      <div class="es-name"><TextInput label="Reverse label" placeholder="reads from the second" bind:value={draft.reverse} /></div>
    {/if}
    <Switch label="Same both ways" bind:checked={draft.symmetric} />
    <Button variant="primary" type="submit">Add kind</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete ${doomed?.name ?? 'kind'}?`} description="Only kinds no relation uses can be deleted.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete kind</Button>
  {/snippet}
</Dialog>

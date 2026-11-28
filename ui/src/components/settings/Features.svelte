<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import Switch from '../../ui/Switch.svelte';
  import { FEATURES, features, loadFeatures, setFeature } from '../../lib/features.svelte';

  let error = $state('');
  onMount(loadFeatures);
  const change = (name: (typeof FEATURES)[number]['name'], on: boolean) =>
    setFeature(name, on).then(() => (error = ''), (e) => (error = (e as { message?: string }).message ?? String(e)));
</script>

<SectionHead lede="Switch off what you do not use. Its sidebar entries, pages, commands and cards disappear; your data stays." />

<div class="st-card">
  <ul class="st-list">
    {#each FEATURES as f (f.name)}
      <li class="feature">
        <div class="st-field">
          <span>{f.label}</span>
          <span class="st-hint">{f.blurb}</span>
        </div>
        <Switch label={f.label} bind:checked={() => features[f.name], (v) => void change(f.name, v)} />
      </li>
    {/each}
  </ul>
  {#if error}<p role="alert" class="st-error">{error}</p>{/if}
  <span class="st-hint">Applies at once; no need to save. From the terminal: taskapp features set NAME on|off.</span>
</div>

<style>
  .feature { display: flex; align-items: center; justify-content: space-between; gap: var(--s-7); }
</style>

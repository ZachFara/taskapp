<script lang="ts">
  import { onMount } from 'svelte';
  import type { Issue, IssuePatch } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import OptionList from '../detail/OptionList.svelte';
  import PropertyRow from '../detail/PropertyRow.svelte';
  import { pointsOf, pointsOn, readEstimation, SCALES, type Estimation, type ScaleName } from './estimation';

  // The Points line of the property plate: the workspace's scale as choices, or any number to one decimal.
  // Hidden when the workspace estimates in time only, or not at all.
  let { issue, save, openRow = $bindable(null) }: { issue: Issue; save: (p: IssuePatch) => void; openRow?: string | null } = $props();

  let estimation = $state<Estimation>('both');
  let scale = $state<ScaleName>('fibonacci');
  let draft = $state('');
  const points = $derived(pointsOf(issue));
  const set = (points: number | null) => save({ points } as IssuePatch);

  onMount(async () => ({ estimation, scale } = await readEstimation()));

  function submit(close: () => void) {
    const n = Math.round(Number(draft) * 10) / 10;
    if (draft.trim() === '') return (close(), set(null));
    if (!Number.isFinite(n) || n < 0) return;
    close();
    set(n);
  }
</script>

{#if pointsOn(estimation)}
  <PropertyRow id="points" name="Points" cmd="issue.points" bind:openRow>
    {#snippet value()}{#if points !== null}{points}{:else}<span class="none">None</span>{/if}{/snippet}
    {#snippet editor(close)}
      <div class="stack">
        <OptionList label="Points" none="No points" value={points} options={SCALES[scale].map((s) => ({ id: s.value, name: s.label }))} onpick={(id) => { close(); set(id === null ? null : Number(id)); }} />
        <form onsubmit={(e) => { e.preventDefault(); submit(close); }}>
          <TextInput type="number" label="Points value" value={String(points ?? '')} oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)} />
          <Button type="submit" variant="primary">Set</Button>
        </form>
      </div>
    {/snippet}
  </PropertyRow>
{/if}

<style>
  .none { color: var(--muted); font-weight: 400; }
  .stack { display: grid; gap: var(--s-4); }
  form { display: flex; gap: var(--s-4); align-items: center; }
</style>

<script lang="ts">
  import type { PropertyDef, Value } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import Pill from '../../ui/Pill.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { isDate } from './format';
  import OptionList from './OptionList.svelte';
  import PropertyRow from './PropertyRow.svelte';

  // One plate row per custom property, for an issue or a project: `values` is its `properties` map.
  let { defs, values, setProp, openRow = $bindable(null) }: {
    defs: PropertyDef[];
    values: Record<string, Value>;
    setProp: (key: string, value: Value) => void;
    openRow?: string | null;
  } = $props();
  let draft = $state('');
</script>

{#snippet none()}<span class="none">None</span>{/snippet}

  {#each defs as d (d.key)}
    {@const v = values[d.key] ?? null}
    <PropertyRow id={`prop:${d.key}`} name={d.name} bind:openRow>
      {#snippet value()}
        {#if d.kind === 'Checkbox'}{v === true ? 'Yes' : 'No'}
        {:else if Array.isArray(v)}{#each v as x (x)}<Pill>{x}</Pill>{:else}{@render none()}{/each}
        {:else if v === null || v === ''}{@render none()}{:else}{v}{/if}
      {/snippet}
      {#snippet editor(close)}
        {#if d.kind === 'Select'}
          <OptionList label={d.name} none="None" value={typeof v === 'string' ? v : null} options={d.options.map((o) => ({ id: o.value, name: o.value }))} onpick={(id) => { close(); setProp(d.key, id as string | null); }} />
        {:else if d.kind === 'MultiSelect'}
          {@const have = Array.isArray(v) ? v : []}
          <ul class="checks" aria-label={d.name}>
            {#each d.options as o (o.value)}
              <li><Checkbox label={o.value} bind:checked={() => have.includes(o.value), (on) => setProp(d.key, on ? [...have, o.value] : have.filter((x) => x !== o.value))} /> {o.value}</li>
            {/each}
          </ul>
        {:else if d.kind === 'Checkbox'}
          <span class="inline"><Checkbox label={d.name} bind:checked={() => v === true, (on) => setProp(d.key, on)} /> {d.name}</span>
        {:else if d.kind === 'Date'}
          <TextInput type="date" label={d.name} value={typeof v === 'string' ? v : ''} oninput={(e) => { const t = (e.currentTarget as HTMLInputElement).value; if (isDate(t)) setProp(d.key, t || null); }} />
        {:else}
          <form class="stack" onsubmit={(e) => { e.preventDefault(); close(); setProp(d.key, draft.trim() === '' ? null : d.kind === 'Number' ? Number(draft) : draft); }}>
            <TextInput type={d.kind === 'Number' ? 'number' : 'text'} label={`${d.name} value`} value={v === null ? '' : String(v)} oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)} />
            <Button type="submit" variant="primary">Set</Button>
          </form>
        {/if}
      {/snippet}
    </PropertyRow>
  {/each}

<style>
  .none { color: var(--muted); font-weight: 400; }
  .stack { display: grid; gap: var(--s-4); }
  .inline { display: flex; align-items: center; gap: var(--s-4); }
  .checks { list-style: none; display: grid; gap: var(--s-4); }
  .checks li { display: flex; align-items: center; gap: var(--s-4); }
</style>

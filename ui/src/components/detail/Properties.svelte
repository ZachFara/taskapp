<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, PropertyDef, Value } from '../../lib/bindings';

  let { issue, act }: { issue: Issue; act: (p: Promise<unknown>) => void } = $props();
  let defs = $state<PropertyDef[]>([]);

  $effect(() => {
    api.properties.listDefs().then((d) => (defs = d));
  });
  // A definition scoped to another project does not apply to this issue.
  const visible = $derived(defs.filter((d) => d.project_id === null || d.project_id === issue.project_id));
  const set = (key: string, value: Value) => act(api.properties.setValue(issue.id, key, value));
  const text = (e: Event) => (e.currentTarget as HTMLInputElement).value || null;
</script>

<section aria-label="Properties">
  <h3>Properties</h3>
  {#each visible as d (d.key)}
    {@const v = issue.properties[d.key] ?? null}
    <label>{d.name}
      {#if d.kind === 'Number'}
        <input type="number" value={v ?? ''} onchange={(e) => set(d.key, e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} />
      {:else if d.kind === 'Date'}
        <input type="date" value={v ?? ''} onchange={(e) => set(d.key, text(e))} />
      {:else if d.kind === 'Checkbox'}
        <input type="checkbox" checked={v === true} onchange={(e) => set(d.key, e.currentTarget.checked)} />
      {:else if d.kind === 'Select'}
        <select onchange={(e) => set(d.key, e.currentTarget.value || null)}>
          <option value="">None</option>
          {#each d.options as o}<option value={o.value} selected={v === o.value}>{o.value}</option>{/each}
        </select>
      {:else if d.kind === 'MultiSelect'}
        {#each d.options as o}
          {@const have = Array.isArray(v) ? v : []}
          <label><input type="checkbox" checked={have.includes(o.value)}
            onchange={(e) => set(d.key, e.currentTarget.checked ? [...have, o.value] : have.filter((x) => x !== o.value))} />{o.value}</label>
        {/each}
      {:else}
        <input value={v ?? ''} onchange={(e) => set(d.key, text(e))} />
      {/if}
    </label>
  {/each}
</section>

<style>
  section { display: grid; gap: 6px; }
  label { display: flex; gap: 8px; align-items: center; font-size: 13px; }
</style>

<script lang="ts">
  import type { Op, Value } from '../../lib/bindings';
  import Checkbox from '../../ui/Checkbox.svelte';
  import Icon from '../../ui/Icon.svelte';
  import Popover from '../../ui/Popover.svelte';
  import Select from '../../ui/Select.svelte';
  import Switch from '../../ui/Switch.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import type { Lookup } from './lookup';
  import { multi, noValue, opsFor, type Chip } from './model';

  let { chip, lookup, open = $bindable(false), onchange, onremove }: {
    chip: Chip; lookup: Lookup; open?: boolean; onchange: (c: Chip) => void; onremove: () => void;
  } = $props();

  const words: Record<Op, string> = {
    Eq: 'is', Neq: 'is not', In: 'is any of', NotIn: 'is none of', Contains: 'contains', Lt: 'before', Lte: 'up to',
    Gt: 'after', Gte: 'from', IsEmpty: 'is empty', IsNotEmpty: 'is set',
  };
  const kind = $derived(lookup.kind(chip.field));
  const choices = $derived(lookup.choices(chip.field));
  // A pick-list field always edits as "any of"; the single-value ops stay for dates, numbers and text.
  const ops = $derived(choices.length ? (['In', 'NotIn'] as Op[]) : opsFor(kind));
  const name = $derived(lookup.name(chip.field));
  const picked = $derived(([] as Value[]).concat(chip.value ?? []));
  const shown = $derived(
    noValue(chip.op) ? '' : choices.length
      ? picked.map((v) => choices.find((c) => c.value === v)?.label ?? String(v)).join(', ')
      : kind === 'Checkbox' ? (chip.value === true ? 'yes' : 'no') : String(chip.value ?? ''),
  );
  const set = (patch: Partial<Chip>) => onchange({ ...chip, ...patch });
  function setOp(op: string) {
    const next = op as Op;
    set({ op: next, value: multi(next) ? picked : multi(chip.op) ? (picked[0] ?? null) : chip.value });
  }
  const toggle = (v: Value, on: boolean) => set({ value: on ? [...picked, v] : picked.filter((x) => x !== v) });
</script>

<span class="chip" data-open={open ? '' : undefined} data-pending={!noValue(chip.op) && !shown ? '' : undefined}>
  <Popover bind:open>
    {#snippet trigger(props)}
      <button {...props} type="button" class="face" aria-label={`${name} ${words[chip.op]} ${shown}`.trim()}>
        <b>{name}</b><span class="op">{words[chip.op]}</span>{#if shown}<span class="val">{shown}</span>{/if}
      </button>
    {/snippet}
    <div class="editor">
      <Select label={`${name} operator`} items={ops.map((o) => ({ value: o, label: words[o] }))} bind:value={() => chip.op, setOp} />
      {#if noValue(chip.op)}
        <!-- nothing to pick -->
      {:else if choices.length}
        <ul class="opts">
          {#each choices as c (c.value)}
            <li><Checkbox label={c.label} bind:checked={() => picked.includes(c.value), (on) => toggle(c.value, on)} /></li>
          {/each}
        </ul>
      {:else if kind === 'Checkbox'}
        <Switch label={name} bind:checked={() => chip.value === true, (on) => set({ value: on })} />
      {:else}
        <TextInput
          label={`${name} value`}
          type={kind === 'Date' ? 'date' : kind === 'Number' ? 'number' : 'text'}
          value={String(chip.value ?? '')}
          oninput={(e) => {
            const v = (e.currentTarget as HTMLInputElement).value;
            set({ value: kind === 'Number' ? (v === '' ? null : Number(v)) : v });
          }}
        />
      {/if}
    </div>
  </Popover>
  <button type="button" class="x" aria-label={`Remove filter ${name}`} onclick={onremove}><Icon name="close" size={12} /></button>
</span>

<style>
  .chip {
    display: inline-flex; align-items: stretch; height: var(--row-h-compact); border: 1px solid var(--control);
    border-radius: var(--r-ctl); background: var(--card); font-size: var(--fs-sm);
    transition: border-color var(--dur-1) var(--ease), background var(--dur-1) var(--ease);
  }
  .chip:hover, .chip[data-open] { border-color: var(--stone); background: var(--selection); }
  .chip[data-pending] { border-style: dashed; }
  .face, .x { border: 0; background: transparent; color: inherit; display: inline-flex; align-items: center; }
  .face { gap: var(--s-3); padding: 0 var(--s-4); }
  b { font-weight: 700; }
  .op { color: var(--muted); }
  .val { font-weight: 600; color: var(--stone); max-width: 18ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .x { padding: 0 var(--s-3); color: var(--muted); border-left: 1px solid var(--border); }
  .x:hover { color: var(--signal); }
  .editor { display: grid; gap: var(--s-4); }
  .opts { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-2); max-height: 260px; overflow: auto; }
</style>

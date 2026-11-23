<script lang="ts" module>
  export type IconColor = { icon: string | null; color: string | null };
  export const PALETTE = ['gray', 'red', 'orange', 'amber', 'yellow', 'green', 'teal', 'cyan', 'blue', 'indigo', 'purple', 'pink'];
  const EMOJI: Record<string, string[]> = {
    Work: ['📌', '📎', '📝', '📅', '📊', '📈', '💼', '🗂️', '🔧', '🛠️', '💡', '🧪'],
    Nature: ['🌱', '🌲', '🌊', '🔥', '⛰️', '🌙', '☀️', '⭐', '🍀', '🌸', '🍂', '❄️'],
    Things: ['🚀', '🎯', '🏁', '🧭', '🗺️', '🔒', '🔔', '📦', '🎨', '🎧', '🏠', '🚲'],
    Faces: ['😀', '🤔', '😎', '🥳', '😴', '🤯', '🙌', '👍', '👀', '💪', '❤️', '✅'],
  };
</script>

<script lang="ts">
  import { ICON_IDS } from '../../ui/icons';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import Popover from '../../ui/Popover.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import Button from '../../ui/Button.svelte';

  // The one place an entity gets its look. `value.icon` is `icon:<id>` or `emoji:<x>`, `value.color` a palette key.
  // `colorOnly` hides the icon tabs (select-option colours).
  let {
    value,
    onchange,
    label = 'Icon and colour',
    colorOnly = false,
    open = $bindable(false),
  }: { value: IconColor; onchange: (v: IconColor) => void; label?: string; colorOnly?: boolean; open?: boolean } = $props();

  let tab = $state<'icons' | 'emoji'>('icons');
  let query = $state('');
  let pasted = $state('');

  const icons = $derived(ICON_IDS.filter((id) => id.includes(query.trim().toLowerCase())));
  const emit = (patch: Partial<IconColor>) => onchange({ ...value, ...patch });
  const useEmoji = () => {
    const x = pasted.trim();
    if (!x) return;
    emit({ icon: `emoji:${x}` });
    pasted = '';
  };
</script>

<Popover bind:open>
  {#snippet trigger(props)}
    <button {...props} class="chip" type="button" aria-label={label} data-open={open || undefined}>
      <EntityIcon icon={value.icon} color={value.color} size={18} />
    </button>
  {/snippet}
  <div class="picker">
    {#if !colorOnly}
      <div class="seg">
        <button type="button" aria-label="Icons" aria-pressed={tab === 'icons'} onclick={() => (tab = 'icons')}>Icons</button>
        <button type="button" aria-label="Emoji" aria-pressed={tab === 'emoji'} onclick={() => (tab = 'emoji')}>Emoji</button>
      </div>
      {#if tab === 'icons'}
        <TextInput type="search" label="Search icons" placeholder="Search icons" bind:value={query} />
        <div class="grid" role="group" aria-label="Icons">
          {#each icons as id (id)}
            <button type="button" class="cell" aria-label={id} aria-pressed={value.icon === `icon:${id}`} onclick={() => emit({ icon: `icon:${id}` })}>
              <EntityIcon icon={`icon:${id}`} color={value.color} />
            </button>
          {:else}
            <p class="none">No icon matches.</p>
          {/each}
        </div>
      {:else}
        <form class="paste" onsubmit={(e) => { e.preventDefault(); useEmoji(); }}>
          <TextInput label="Paste an emoji" placeholder="Paste any emoji" bind:value={pasted} />
          <Button type="submit" tooltip={{ label: 'Use emoji' }}>Use</Button>
        </form>
        {#each Object.entries(EMOJI) as [group, list] (group)}
          <div class="cap">{group}</div>
          <div class="grid" role="group" aria-label={group}>
            {#each list as e (e)}
              <button type="button" class="cell" data-emoji={e} aria-label={`Emoji ${e}`} aria-pressed={value.icon === `emoji:${e}`} onclick={() => emit({ icon: `emoji:${e}` })}>
                <EntityIcon icon={`emoji:${e}`} />
              </button>
            {/each}
          </div>
        {/each}
      {/if}
    {/if}
    <div class="cap">Colour</div>
    <div class="swatches" role="group" aria-label="Colours">
      {#each PALETTE as key (key)}
        <button type="button" class="sw" style:--tone={`var(--c-${key})`} aria-label={key} aria-pressed={value.color === key} onclick={() => emit({ color: key })}></button>
      {/each}
    </div>
    <button type="button" class="clear" aria-label="Clear icon and colour" onclick={() => onchange({ icon: null, color: null })}>Clear</button>
  </div>
</Popover>

<style>
  .chip {
    width: var(--row-h-compact);
    height: var(--row-h-compact);
    display: inline-grid;
    place-items: center;
    padding: 0;
    border: 1px solid transparent; /* the outline shows on hover, so a column of them stays quiet */
    border-radius: var(--r-ctl);
    background: transparent;
    transition: background var(--dur-1) var(--ease), border-color var(--dur-1) var(--ease);
  }
  .chip:hover, .chip[data-open] { background: var(--active); border-color: var(--control); }
  .picker { width: 296px; display: grid; gap: var(--s-4); }
  .seg { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s-1); padding: var(--s-1); background: var(--raised); border-radius: var(--r-ctl); }
  .seg button { height: var(--row-h-compact); border: 0; border-radius: var(--r-sm); background: transparent; color: var(--muted); font-weight: 600; }
  .seg button[aria-pressed='true'] { background: var(--sheet); color: var(--ink); }
  .grid { display: grid; grid-template-columns: repeat(8, 1fr); gap: var(--s-1); max-height: 168px; overflow: auto; }
  .cell { height: var(--row-h); border: 0; border-radius: var(--r-ctl); background: transparent; display: grid; place-items: center; padding: 0; }
  .cell:hover { background: var(--active); }
  .cell[aria-pressed='true'] { background: var(--selection); }
  .paste { display: grid; grid-template-columns: 1fr auto; gap: var(--s-3); }
  .cap { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); }
  .none { grid-column: 1 / -1; color: var(--muted); }
  .swatches { display: grid; grid-template-columns: repeat(6, 1fr); gap: var(--s-3); }
  .sw { height: var(--s-9); border-radius: var(--r-ctl); border: 2px solid transparent; background: var(--tone); padding: 0; }
  .sw[aria-pressed='true'] { border-color: var(--ink); outline: 2px solid var(--sheet); outline-offset: -4px; }
  .clear { justify-self: start; border: 0; background: transparent; color: var(--muted); padding: 0; text-decoration: underline; }
  .clear:hover { color: var(--ink); }
</style>

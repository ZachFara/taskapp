<script lang="ts">
  import type { StatusCategory } from '../lib/bindings';
  import { ICON_IDS } from '../ui/icons';
  import Button from '../ui/Button.svelte';
  import Checkbox from '../ui/Checkbox.svelte';
  import ContextMenu from '../ui/ContextMenu.svelte';
  import ListRow from '../ui/ListRow.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import GroupHeader from '../ui/GroupHeader.svelte';
  import Icon from '../ui/Icon.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Popover from '../ui/Popover.svelte';
  import PriorityGlyph from '../ui/PriorityGlyph.svelte';
  import Select from '../ui/Select.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import Spinner from '../ui/Spinner.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import Switch from '../ui/Switch.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextArea from '../ui/TextArea.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import Toast from '../ui/Toast.svelte';
  import Tooltip from '../ui/Tooltip.svelte';

  const roles = [
    ['frame', 'frame-raised', 'frame-inset', 'frame-ink', 'frame-muted'],
    ['sheet', 'raised', 'active', 'selection', 'card'],
    ['band-1', 'band-2', 'band-3', 'band-4', 'band-ink', 'band-count'],
    ['ink', 'muted', 'border', 'control', 'focus'],
    ['accent', 'accent-ink', 'stone', 'signal', 'positive', 'warning'],
    ['tip-bg', 'tip-ink'],
  ];
  const palette = ['gray', 'red', 'orange', 'amber', 'yellow', 'green', 'teal', 'cyan', 'blue', 'indigo', 'purple', 'pink'];
  const categories: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];
  const sizes = [
    ['fs-xs', 'Eleven: counts, key hints'],
    ['fs-sm', 'Twelve: secondary, meta'],
    ['fs-md', 'Thirteen: body and rows'],
    ['fs-lg', 'Fifteen: dialog titles'],
    ['fs-title', 'Twenty: page titles'],
    ['fs-display', 'Twenty-eight'],
  ];
  const spaces = ['s-1', 's-2', 's-3', 's-4', 's-5', 's-6', 's-7', 's-8', 's-9', 's-10'];
  const radii = ['r-sm', 'r-ctl', 'r-pop', 'r-sheet', 'r-round'];

  const asked = new URLSearchParams(location.search).get('theme');
  let theme = $state<'system' | 'light' | 'dark'>(asked === 'light' || asked === 'dark' ? asked : 'system');
  $effect(() => {
    const el = document.documentElement;
    if (theme === 'system') el.removeAttribute('data-theme');
    else el.dataset.theme = theme;
  });

  let layout = $state('list');
  let bare = $state(false);
  let dialogSize = $state<'sm' | 'md' | 'lg'>('md');
  let rowPicked = $state(1);
  let heading = $state('Editable title');
  let lastChange = $state('none');
  let checked = $state(true);
  let mixed = $state(false);
  let on = $state(true);
  let sel = $state('started');
  let name = $state('Rework onboarding');
  let notes = $state('');
  let dialog = $state(false);
  let collapsed = $state(false);
  const statusItems = [
    { value: 'backlog', label: 'Backlog' },
    { value: 'todo', label: 'Todo' },
    { value: 'started', label: 'In Progress' },
    { value: 'done', label: 'Done' },
  ];
</script>

<div class="cairn-root" data-on-frame>
  <div class="frame">
    <div class="brand">
      <Icon name="cairn" size={24} />
      <span class="wordmark">Cairn</span>
      <span class="sub">Design system</span>
      <div class="grow"></div>
      <Tabs
        bind:value={theme}
        label="Theme"
        tabs={[
          { id: 'system', label: 'System', icon: 'settings' },
          { id: 'light', label: 'Light', icon: 'sun' },
          { id: 'dark', label: 'Dark', icon: 'moon' },
        ]}
      />
    </div>

    <div class="sheet">
      <section>
        <h2>Colour roles</h2>
        <p class="note">Components use roles only. A theme is a new block of these values.</p>
        {#each roles as row}
          <div class="swatches">
            {#each row as r (r)}
              <div class="sw"><span class="chip" style:background={`var(--${r})`}></span><span>{r}</span></div>
            {/each}
          </div>
        {/each}
      </section>

      <section>
        <h2>Label palette</h2>
        <p class="note">The twelve backend keys, checked at 3:1 on the sheet and band steps 1 and 2 in both themes. Shown on each surface.</p>
        {#each ['sheet', 'raised', 'band-1', 'band-2', 'band-3', 'band-4'] as bg (bg)}
          <div class="pal" style:background={`var(--${bg})`}>
            <span class="bgname">{bg}</span>
            {#each palette as c (c)}
              <span class="pc" style:color={`var(--c-${c})`}><EntityIcon icon="icon:folder" color={c} /><i>{c}</i></span>
            {/each}
          </div>
        {/each}
      </section>

      <section>
        <h2>Status circles and priority bars</h2>
        <p class="note">Status circles by category, priority bars by level. Sixteen is the working size; thirty-two is for review.</p>
        <div class="glyphs">
          {#each categories as c (c)}
            <div class="g"><StatusGlyph category={c} size={16} /><StatusGlyph category={c} size={32} /><span>{c}</span></div>
          {/each}
        </div>
        <div class="glyphs">
          {#each [0, 1, 2, 3, 4] as p (p)}
            <div class="g"><PriorityGlyph level={p} size={16} /><PriorityGlyph level={p} size={32} /><span>{['None', 'Low', 'Medium', 'High', 'Urgent'][p]}</span></div>
          {/each}
        </div>
        <div class="onbands">
          {#each [1, 2, 3, 4] as b (b)}
            <div class="strip" style:background={`var(--band-${b})`}>
              {#each categories as c (c)}<StatusGlyph category={c} />{/each}
              {#each [0, 1, 2, 3, 4] as p (p)}<PriorityGlyph level={p} />{/each}
            </div>
          {/each}
        </div>
      </section>

      <section>
        <h2>Icons</h2>
        <p class="note">{ICON_IDS.length} icons, one 16px set. Ids are the icon:&lt;id&gt; values the backend stores.</p>
        <div class="icons">
          {#each ICON_IDS as id (id)}<div class="ic"><Icon name={id} /><span>{id}</span></div>{/each}
          <div class="ic"><EntityIcon icon="emoji:🚀" /><span>emoji:x</span></div>
        </div>
      </section>

      <section>
        <h2>Type, space, shape, motion</h2>
        <div class="type">
          {#each sizes as [t, text] (t)}<div style:font-size={`var(--${t})`} class="tl"><b>{text}</b><span>{t}</span></div>{/each}
          <div class="tl"><b>0123456789 3h 1d Nov 24</b><span>tabular figures by default</span></div>
        </div>
        <div class="row-wrap">
          {#each spaces as s (s)}<div class="sp"><span class="bar" style:width={`var(--${s})`}></span><i>{s}</i></div>{/each}
        </div>
        <div class="row-wrap">
          {#each radii as r (r)}<div class="rd" style:border-radius={`var(--${r})`}><i>{r}</i></div>{/each}
          <div class="rd float"><i>shadow-float</i></div>
          <div class="rd mo"><i>dur 80 / 140 / 220</i></div>
        </div>
      </section>

      <section>
        <h2>Buttons</h2>
        <div class="line">
          <Button variant="primary" icon="plus" kbd="C">New issue</Button>
          <Button variant="primary">Save</Button>
          <Button variant="quiet" icon="filter">Filter</Button>
          <Button variant="quiet">Cancel</Button>
          <Button variant="danger" icon="trash">Delete</Button>
          <Button variant="primary" disabled>Disabled</Button>
          <Button variant="quiet" disabled>Disabled</Button>
          <Button variant="primary" tooltip={{ label: 'Create issue', shortcut: 'C' }}>With tooltip</Button>
        </div>
        <div class="line">
          <IconButton icon="search" label="Search" shortcut={['⌘', 'K']} />
          <IconButton icon="panel" label="Toggle panel" pressed={true} />
          <IconButton icon="more" label="More" />
          <IconButton icon="trash" label="Delete" disabled />
          <Kbd keys={['⌘', 'K']} /><Kbd keys="J" /><Kbd keys="↵" />
        </div>
      </section>

      <section>
        <h2>Fields</h2>
        <div class="fields">
          <TextInput bind:value={name} label="Title" placeholder="Issue title" />
          <TextInput value="" label="Empty" placeholder="Placeholder" />
          <TextInput value="Bad value" label="Invalid" invalid={true} />
          <TextInput value="Locked" label="Disabled" disabled={true} />
          <Select bind:value={sel} items={statusItems} label="Status" />
          <Select value="" items={statusItems} label="Empty select" placeholder="Pick a status" />
          <TextArea bind:value={notes} label="Notes" placeholder="Write a description" rows={3} />
          <div class="line">
            <Checkbox bind:checked label="Checked" />
            <Checkbox checked={false} label="Unchecked" />
            <Checkbox bind:checked={mixed} indeterminate={true} label="Mixed" />
            <Checkbox checked={true} disabled label="Disabled" />
            <Switch bind:checked={on} label="On" />
            <Switch checked={false} label="Off" />
            <Switch checked disabled label="Disabled" />
          </div>
        </div>
      </section>

      <section>
        <h2>Overlays</h2>
        <div class="line">
          <Menu>
            {#snippet trigger(props)}<Button {...props} variant="quiet" icon="more">Menu</Button>{/snippet}
            <MenuItem label="Rename" icon="edit" shortcut="R" />
            <MenuItem label="Copy link" icon="link" shortcut={['⌘', 'L']} />
            <MenuItem label="Archive" icon="archive" disabled />
            <MenuItem label="Delete" icon="trash" danger />
          </Menu>
          <Popover>
            {#snippet trigger(props)}<Button {...props} variant="quiet" icon="calendar">Popover</Button>{/snippet}
            <div class="pop"><b>Due date</b><TextInput type="date" label="Due date" value="" /></div>
          </Popover>
          <Button variant="quiet" onclick={() => (dialog = true)}>Dialog</Button>
          <Button variant="quiet" onclick={() => { dialogSize = 'md'; bare = true; }}>Bare dialog md</Button>
          <Button variant="quiet" onclick={() => { dialogSize = 'lg'; bare = true; }}>Bare dialog lg</Button>
          <ContextMenu>
            {#snippet area()}<div class="ctx">Right-click here</div>{/snippet}
            <MenuItem label="Rename" icon="edit" shortcut="R" />
            <MenuItem label="Delete" icon="trash" danger />
          </ContextMenu>
          <Tooltip label="Hover or focus me" shortcut="?">
            {#snippet children(props)}<button {...props} class="plain">Tooltip (--delay-tooltip)</button>{/snippet}
          </Tooltip>
        </div>
        <div class="line">
          <Toast tone="info" ondismiss={() => {}}>Moved ABC-12 to In Progress {#snippet action()}Undo{/snippet}</Toast>
          <Toast tone="success">Saved</Toast>
          <Toast tone="error">Could not reach the data store</Toast>
        </div>
        <div class="line">
          <Pill>design</Pill><Pill color="blue">Mobile app</Pill><Pill color="orange">ux</Pill><Pill color="green">release</Pill><Pill color="pink">a11y</Pill>
          <Spinner /><Skeleton width="160px" /><Skeleton width="80px" height="var(--s-5)" />
        </div>
      </section>

      <section>
        <h2>Page header</h2>
        <div class="demo">
          <PageHeader title="Active" icon="icon:layers" color="green">
            {#snippet tabs()}
              <Tabs bind:value={layout} tabs={[{ id: 'list', label: 'List' }, { id: 'board', label: 'Board' }, { id: 'table', label: 'Table' }, { id: 'timeline', label: 'Timeline' }]} />
            {/snippet}
            {#snippet actions()}<Button variant="primary" icon="plus" kbd="C">New issue</Button>{/snippet}
            {#snippet filters()}<Button variant="quiet" icon="filter">Filter</Button><Pill>Status is not Done</Pill>{/snippet}
          </PageHeader>
        </div>
      </section>

      <section>
        <h2>Editable page header, list rows, field callbacks</h2>
        <div class="demo">
          <PageHeader title={heading} editable onrename={(v) => (heading = v)} />
          <PageHeader>
            {#snippet title()}Title snippet <Pill>slot</Pill>{/snippet}
          </PageHeader>
          {#each [1, 2, 3] as n}
            <ListRow focused={n === 2} selected={n === rowPicked} onpick={() => (rowPicked = n)}>Cycle {n}</ListRow>
          {/each}
        </div>
        <div class="line">
          <TextInput type="time" label="Time" value="09:00" onchange={(v) => (lastChange = v)} />
          <TextInput type="number" label="Number" value="3" onchange={(v) => (lastChange = v)} />
          <Checkbox checked={false} label="Callback" onchange={(c) => (lastChange = String(c))} />
          <span class="note">onchange: {lastChange}</span>
        </div>
      </section>

      <section>
        <h2>Group headers (legend bands)</h2>
        <div class="demo">
          <GroupHeader name="Backlog" count={5} category="Backlog" />
          <GroupHeader name="Todo" count={6} category="Unstarted" />
          <GroupHeader name="In Progress" count={4} category="Started" />
          <GroupHeader name="Done" count={3} category="Completed" {collapsed} ontoggle={() => (collapsed = !collapsed)} />
          <GroupHeader name="Canceled" count={1} category="Canceled" />
          <GroupHeader name="Mobile app" count={4} tone={2}>
            {#snippet leading()}<EntityIcon icon="icon:layers" color="blue" />{/snippet}
          </GroupHeader>
        </div>
      </section>

      <section>
        <h2>Empty state</h2>
        <div class="demo">
          <EmptyState title="Nothing here yet" hint="Issues that match this view show up here. Press C to add the first one.">
            {#snippet action()}<Button variant="primary" icon="plus" kbd="C">New issue</Button>{/snippet}
          </EmptyState>
        </div>
      </section>
    </div>
  </div>

  <Dialog bind:open={dialog} title="Delete this view?" description="Issues in it are kept. This cannot be undone.">
    {#snippet actions()}
      <Button variant="quiet" onclick={() => (dialog = false)}>Cancel</Button>
      <Button variant="danger" onclick={() => (dialog = false)}>Delete view</Button>
    {/snippet}
  </Dialog>
  <Dialog bind:open={bare} size={dialogSize} label="Bare dialog">
    <div class="pop"><b>Bare {dialogSize} dialog</b><Button variant="quiet" onclick={() => (bare = false)}>Close</Button></div>
  </Dialog>
</div>

<style>
  /* Covers the old shell, which still wraps every route until the new one lands. */
  .cairn-root { position: fixed; inset: 0; z-index: var(--z-sticky); display: flex; flex-direction: column; overflow: hidden; }
  .frame { display: flex; flex-direction: column; min-height: 0; flex: 1; }
  .brand { height: var(--topbar-h); flex: none; display: flex; align-items: center; gap: var(--s-5); padding: 0 var(--s-7); color: var(--frame-ink); }
  .wordmark { font-size: var(--fs-wordmark); font-weight: 700; letter-spacing: -0.02em; font-stretch: 90%; }
  .sub { color: var(--frame-muted); font-weight: 500; margin-left: var(--s-4); }
  .grow { flex: 1; }
  .brand :global(.cx-tab) { color: var(--frame-muted); }
  .brand :global(.cx-tab[data-state='active']) { color: var(--frame-ink); border-bottom-color: var(--frame-ink); }
  .brand :global(.cx-tab:hover) { color: var(--frame-ink); }
  .sheet {
    flex: 1;
    min-height: 0;
    overflow: auto;
    margin: 0 var(--sheet-inset) var(--sheet-inset);
    padding: var(--s-8) var(--s-9) var(--s-10);
    background: var(--sheet);
    color: var(--ink);
    border: 1px solid var(--border);
    border-radius: var(--r-sheet);
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: var(--s-9);
  }
  section { display: grid; gap: var(--s-6); max-width: 1100px; }
  h2 { font-size: var(--fs-title); font-weight: 700; letter-spacing: -0.02em; }
  .note { color: var(--muted); margin-top: calc(var(--s-4) * -1); }
  .swatches { display: flex; gap: var(--s-4); flex-wrap: wrap; }
  .sw { display: grid; gap: var(--s-2); width: 108px; font-size: var(--fs-sm); color: var(--muted); }
  .chip { height: 44px; border-radius: var(--r-ctl); border: 1px solid var(--border); }
  .pal { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-2) var(--s-6); padding: var(--s-4) var(--s-6); border-radius: var(--r-ctl); border: 1px solid var(--border); }
  .bgname { width: 52px; font-size: var(--fs-sm); color: var(--ink); font-weight: 600; }
  .pc { display: inline-flex; align-items: center; gap: var(--s-2); font-size: var(--fs-sm); font-weight: 600; }
  .pc i { font-style: normal; }
  .glyphs { display: flex; gap: var(--s-8); flex-wrap: wrap; }
  .g { display: flex; align-items: center; gap: var(--s-5); padding: var(--s-4) var(--s-6); background: var(--raised); border-radius: var(--r-ctl); min-width: 160px; }
  .g span { color: var(--muted); font-size: var(--fs-sm); margin-left: var(--s-2); }
  .onbands { display: grid; gap: 0; border-radius: var(--r-ctl); overflow: hidden; width: max-content; }
  .strip { display: flex; gap: var(--s-6); padding: var(--s-3) var(--s-6); }
  .icons { display: grid; grid-template-columns: repeat(auto-fill, minmax(108px, 1fr)); gap: var(--s-2); }
  .ic { display: grid; justify-items: center; gap: var(--s-3); padding: var(--s-5) var(--s-2); border-radius: var(--r-ctl); background: var(--raised); font-size: var(--fs-xs); color: var(--muted); }
  .ic :global(.icon), .ic :global(.entity) { color: var(--ink); }
  .type { display: grid; gap: var(--s-2); }
  .tl { display: flex; justify-content: space-between; align-items: baseline; border-bottom: 1px solid var(--border); padding-bottom: var(--s-2); max-width: 720px; }
  .tl span { font-size: var(--fs-sm); color: var(--muted); }
  .row-wrap { display: flex; gap: var(--s-7); flex-wrap: wrap; align-items: end; }
  .sp { display: grid; gap: var(--s-2); }
  .bar { height: var(--s-4); background: var(--stone); display: block; border-radius: var(--r-sm); }
  .sp i, .rd i { font-style: normal; font-size: var(--fs-xs); color: var(--muted); }
  .rd { width: 96px; height: 64px; display: grid; place-items: end start; padding: var(--s-3); background: var(--raised); border: 1px solid var(--control); }
  .rd.float { box-shadow: var(--shadow-float); border-color: var(--border); border-radius: var(--r-pop); background: var(--sheet); }
  .rd.mo { border-radius: var(--r-pop); background: var(--band-2); width: 140px; transition: transform var(--dur-3) var(--ease); }
  .rd.mo:hover { transform: translateX(var(--s-7)); }
  .line { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-5); }
  .fields { display: grid; grid-template-columns: repeat(2, minmax(0, 320px)); gap: var(--s-5) var(--s-7); }
  .fields .line, .fields :global(textarea) { grid-column: 1 / -1; }
  .ctx { padding: var(--s-7); border: 1px dashed var(--control); border-radius: var(--r-ctl); color: var(--muted); }
  .pop { display: grid; gap: var(--s-4); }
  .plain { border: 0; background: transparent; color: var(--ink); text-decoration: underline dotted; padding: var(--s-2); }
  .demo { display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: var(--r-ctl); overflow: hidden; background: var(--sheet); }
</style>

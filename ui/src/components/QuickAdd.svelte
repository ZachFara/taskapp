<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import type { Issue, IssueTemplate, ParsedQuickAdd } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import { projects, refreshAll } from '../lib/stores.svelte';
  import Button from '../ui/Button.svelte';
  import Contour from '../ui/Contour.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import PriorityGlyph from '../ui/PriorityGlyph.svelte';

  // `project_slug` and `status_id` prefill from the screen that opened the dialog (a group band, a project page).
  // `oncreated` hands the owner the new issue and an undo for the whole create, so it can show the toast.
  let {
    onclose,
    oncreated,
    project_slug = null,
    status_id = null,
  }: {
    onclose: () => void;
    oncreated?: (created: { issue: Issue; undo: () => Promise<void> }) => void;
    project_slug?: string | null;
    status_id?: number | null;
  } = $props();

  const PRIORITY = ['None', 'Low', 'Medium', 'High', 'Urgent'];
  const DUE_WORDS = String.raw`(?:today|tomorrow|next week|(?:mon|tue|wed|thu|fri|sat|sun)\w*|\d{4}-\d{2}-\d{2}|in \d+ (?:day|days|week|weeks))`;

  let open = $state(true);
  // The prefill is the starting text only; the dialog owns the field after it opens.
  const prefill = untrack(() => (project_slug ? `#${project_slug} ` : ''));
  let text = $state(prefill);
  let parsed = $state<ParsedQuickAdd | null>(null);
  let error = $state('');
  let templates = $state<IssueTemplate[]>([]);
  let labels = $state<{ name: string }[]>([]);
  let highlight = $state(0);
  let field: HTMLInputElement | undefined = $state();
  let seq = 0;

  void api.templates.list().then((t) => (templates = t)).catch(() => {});
  void api.labels.list().then((l) => (labels = l)).catch(() => {});

  // Typing `/` at the start opens the template list; the list narrows as the slug is typed.
  const slashQuery = $derived(/^\/(\S*)$/.exec(text)?.[1] ?? null);
  const matches = $derived(
    slashQuery === null ? [] : templates.filter((t) => t.slug.startsWith(slashQuery.toLowerCase())),
  );
  const template = $derived(templates.find((t) => text.startsWith(`/${t.slug} `) || text === `/${t.slug}`));

  // Each chip is a Menu trigger: Tab reaches it in order, Enter or Space opens its menu, and the menu rewrites
  // that token in the text, so the field stays the one source of truth.
  const chips = $derived(
    [
      template && { key: 'template', label: `/${template.slug}` },
      parsed?.due && { key: 'due', label: `Due ${parsed.due.Date ?? parsed.due.DateTime?.utc}` },
      parsed?.start_date && { key: 'start', label: `Start ${parsed.start_date}` },
      parsed?.priority != null && { key: 'priority', label: PRIORITY[parsed.priority] ?? `P${parsed.priority}` },
      parsed?.project_slug && { key: 'project', label: `#${parsed.project_slug}` },
      ...(parsed?.label_names ?? []).map((l) => ({ key: 'labels', label: `@${l}` })),
      parsed?.estimate_minutes != null && { key: 'estimate', label: `${parsed.estimate_minutes}m` },
    ].filter((c): c is { key: string; label: string } => !!c),
  );

  // Replaces the token matching `pattern` with `token` (or drops it when `token` is empty). Chip menus use it.
  function setToken(pattern: RegExp, token: string) {
    const rest = text.replace(pattern, ' ').replace(/\s+/g, ' ').trim();
    text = token ? `${rest} ${token} `.trimStart() : `${rest} `.trimStart();
    void oninput();
    field?.focus();
  }

  async function oninput() {
    error = '';
    highlight = 0;
    // Templates can be created while the dialog is open, so the list is read when a slug is being typed.
    if (slashQuery !== null) templates = await api.templates.list().catch(() => templates);
    const mine = ++seq;
    const p = text.trim() && !slashQuery ? await api.issues.parseQuickadd(text).catch(() => null) : null;
    if (mine === seq) parsed = p;
  }

  async function submit(mode: 'close' | 'open' | 'keep') {
    const value = text.trim();
    if (!value || (slashQuery !== null && !template)) return;
    let issue: Issue;
    try {
      issue = await api.issues.createFromQuickadd(value);
      // The quick-add grammar has no status token, so a status prefill is a second write.
      if (status_id !== null) await api.issues.update(issue.id, { status_id });
    } catch (err) {
      // Keep the text so a failed create can be fixed and retried.
      error = (err as { message?: string })?.message ?? String(err);
      return;
    }
    const steps = status_id !== null ? 2 : 1;
    oncreated?.({
      issue,
      undo: async () => {
        for (let i = 0; i < steps; i++) await api.history.undo();
        void refreshAll().catch(() => {});
      },
    });
    void refreshAll().catch(() => {});
    if (mode === 'keep') {
      text = prefill;
      parsed = null;
      return;
    }
    if (mode === 'open') navigate(`/issue/${issue.key}`);
    onclose();
  }

  // Plain Enter and Shift+Enter need the field's own keys; the registry leaves plain keys to fields.
  function onkeydown(e: KeyboardEvent) {
    if (e.isComposing) return;
    if (matches.length && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
      e.preventDefault();
      highlight = (highlight + (e.key === 'ArrowDown' ? 1 : matches.length - 1)) % matches.length;
      return;
    }
    if (e.key !== 'Enter') return;
    e.preventDefault();
    if (matches.length) return pickTemplate(matches[highlight]);
    if (e.metaKey || e.ctrlKey) return submit('open');
    if (e.shiftKey) return submit('keep');
    void submit('close');
  }

  function pickTemplate(t: IssueTemplate) {
    text = `/${t.slug} `;
    void oninput();
    field?.focus();
  }

  $effect(() => {
    if (!open) onclose();
  });
  // Focus is set here rather than with `autofocus`, which the a11y rules reject.
  $effect(() => {
    field?.focus();
  });
</script>

<Dialog bind:open title="New issue">
  <div class="stage" aria-hidden="true"><Contour rings={7} size={180} /></div>
  <div class="field-wrap">
    <input
      bind:this={field}
      data-quickadd
      class="title"
      aria-label="Title, with quick-add tokens"
      placeholder="Describe the issue. p2 #project @label ~30m tomorrow"
      bind:value={text}
      oninput={oninput}
      onkeydown={onkeydown}
      role={matches.length ? 'combobox' : 'textbox'}
      aria-expanded={matches.length ? true : undefined}
      aria-controls={matches.length ? 'quickadd-templates' : undefined}
    />
    {#if matches.length}
      <ul id="quickadd-templates" class="templates" role="listbox" aria-label="Templates">
        {#each matches as t, i (t.id)}
          <li role="none">
            <button
              type="button"
              role="option"
              tabindex="-1"
              aria-selected={i === highlight}
              class="option"
              data-active={i === highlight || undefined}
              onclick={() => pickTemplate(t)}
            >
              <span class="slug">/{t.slug}</span> <span class="name">{t.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if chips.length}
    <div class="chips" role="group" aria-label="Parsed fields">
      {#each chips as c (c.key + c.label)}
        <Menu>
          {#snippet trigger(props)}
            <button {...props} class="chip" type="button" data-chip data-kind={c.key} aria-label={`Edit ${c.key}: ${c.label}`}>
              {#if c.key === 'priority'}<PriorityGlyph level={parsed?.priority ?? 0} size={12} />{/if}
              {c.label}
            </button>
          {/snippet}
          {#if c.key === 'priority'}
            {#each PRIORITY as name, level (name)}
              <MenuItem label={name} onSelect={() => setToken(/(^|\s)p[0-4](?=\s|$)/i, level ? `p${level}` : '')} />
            {/each}
          {:else if c.key === 'project'}
            {#each projects as p (p.slug)}
              <MenuItem label={p.name} icon="project" onSelect={() => setToken(/(^|\s)#[\w-]+/, `#${p.slug}`)} />
            {/each}
            <MenuItem label="No project" onSelect={() => setToken(/(^|\s)#[\w-]+/, '')} />
          {:else if c.key === 'due'}
            <MenuItem label="Today" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'today')} />
            <MenuItem label="Tomorrow" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'tomorrow')} />
            <MenuItem label="Next week" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'next week')} />
            <MenuItem label="No due date" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), '')} />
          {:else if c.key === 'start'}
            <MenuItem label="Start today" icon="clock" onSelect={() => setToken(/(^|\s)start\s+\S+(\s+\S+)?/i, 'start today')} />
            <MenuItem label="No start date" onSelect={() => setToken(/(^|\s)start\s+\S+(\s+\S+)?/i, '')} />
          {:else if c.key === 'labels'}
            {#each labels as l (l.name)}
              <MenuItem label={l.name} icon="tag" onSelect={() => setToken(new RegExp(`(^|\\s)@${l.name}(?=\\s|$)`, 'i'), text.toLowerCase().includes(`@${l.name.toLowerCase()}`) ? '' : `@${l.name}`)} />
            {/each}
          {:else if c.key === 'estimate'}
            <MenuItem label="30 minutes" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~30m')} />
            <MenuItem label="2 hours" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~2h')} />
            <MenuItem label="1 day" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~1d')} />
            <MenuItem label="No estimate" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '')} />
          {:else if c.key === 'template'}
            {#each templates as t (t.id)}
              <MenuItem label={t.name} icon="layers" onSelect={() => pickTemplate(t)} />
            {/each}
          {/if}
        </Menu>
      {/each}
    </div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#snippet actions()}
    <span class="hints">
      <span><Kbd keys="enter" /> create</span>
      <span><Kbd keys="shift+enter" /> keep open</span>
      <span><Kbd keys="mod+enter" /> open</span>
    </span>
    <Button variant="primary" icon="plus" onclick={() => submit('close')} disabled={!text.trim()}>Create</Button>
  {/snippet}
</Dialog>

<style>
  .stage { position: absolute; right: calc(var(--s-7) * -1); top: calc(var(--s-7) * -1); color: var(--stone); opacity: 0.35; pointer-events: none; }
  .field-wrap { position: relative; }
  .title {
    width: 100%;
    height: calc(var(--ctl-h) + var(--s-6));
    padding: 0 var(--s-5);
    border: 0;
    border-bottom: 2px solid var(--control);
    background: transparent;
    color: var(--ink);
    font-size: var(--fs-title);
    font-weight: 700;
    letter-spacing: -0.02em;
    transition: border-color var(--dur-1) var(--ease);
  }
  .title::placeholder { color: var(--muted); font-weight: 500; }
  .title:focus-visible { outline-offset: 0; border-color: var(--focus); }
  .templates { position: absolute; z-index: var(--z-popover); top: 100%; left: 0; right: 0; margin: var(--s-2) 0 0; padding: var(--s-2); list-style: none; background: var(--sheet); border: 1px solid var(--control); border-radius: var(--r-pop); box-shadow: var(--shadow-float); }
  .option { display: flex; gap: var(--s-4); width: 100%; height: var(--ctl-h); align-items: center; padding: 0 var(--s-4); border: 0; border-radius: var(--r-sm); background: transparent; color: var(--ink); text-align: left; }
  .option[data-active] { background: var(--active); }
  .slug { font-weight: 700; color: var(--accent); }
  .name { color: var(--muted); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--s-3); }
  .chip { display: inline-flex; align-items: center; gap: var(--s-3); height: var(--row-h-compact); padding: 0 var(--s-4); border: 1px solid var(--control); border-radius: var(--r-round); background: var(--raised); color: var(--ink); font-size: var(--fs-sm); font-weight: 600; transition: background var(--dur-1) var(--ease); }
  .chip:hover { background: var(--active); }
  .error { margin: 0; color: var(--signal); font-size: var(--fs-sm); font-weight: 500; }
  .hints { display: flex; align-items: center; gap: var(--s-5); margin-right: auto; color: var(--muted); font-size: var(--fs-xs); }
  .hints span { display: inline-flex; align-items: center; gap: var(--s-2); }
</style>

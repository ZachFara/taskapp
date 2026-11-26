<script lang="ts">
  import { fieldOn } from '../lib/fields.svelte';
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
  import { humanDate, PRIORITIES, priorityGlyph } from './detail/format';
  import { pointsOn, readEstimation, SCALES, type Estimation, type ScaleName } from './insights/estimation';

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
  let estimation = $state<Estimation>('both');
  let scale = $state<ScaleName>('fibonacci');
  void readEstimation().then((r) => ({ estimation, scale } = r));
  // `points` is not in the generated type until the backend lands, so it is read through a guard.
  const pointsIn = (p: ParsedQuickAdd | null) => ((p as { points?: number | null } | null)?.points ?? null);
  const POINTS_TOKEN = /(^|\s)(\d+(?:\.\d)?pt|pt\d+(?:\.\d)?)(?=\s|$)/i;

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
  // Only the tokens of fields that are switched on.
  const hint = $derived([['priority', 'p2'], ['project', '#project'], ['labels', '@label'], ['estimate', '~30m'], ['due', 'tomorrow']].flatMap(([f, t]) => (fieldOn(f as 'due') ? [t] : [])).map((t) => ` ${t}`).join(''));
  const chips = $derived(
    [
      template && { key: 'template', label: `/${template.slug}` },
      parsed?.due && { key: 'due', label: `Due ${humanDate(parsed.due.Date ?? parsed.due.DateTime!.utc.slice(0, 10))}` },
      parsed?.start_date && { key: 'start', label: `Start ${humanDate(parsed.start_date)}` },
      parsed?.priority != null && { key: 'priority', label: PRIORITIES[parsed.priority] ?? `P${parsed.priority}` },
      parsed?.project_slug && { key: 'project', label: `#${parsed.project_slug}` },
      ...(parsed?.label_names ?? []).map((l) => ({ key: 'labels', label: `@${l}` })),
      parsed?.estimate_minutes != null && { key: 'estimate', label: `${parsed.estimate_minutes}m` },
      pointsIn(parsed) != null && pointsOn(estimation) && { key: 'points', label: `${pointsIn(parsed)} pts` },
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
      if (status_id !== null) {
        // The quick-add grammar has no status token, so the parsed fields and the status go into one write:
        // a template apply or a create, either way one undo step.
        const text = template ? value.slice(template.slug.length + 1).trim() : value;
        const p = await api.issues.parseQuickadd(text);
        const found = await api.labels.list();
        const label_ids = p.label_names.flatMap((n) => found.find((l) => l.name.toLowerCase() === n.toLowerCase())?.id ?? []);
        const fields = {
          title: p.title, due: p.due, start_date: p.start_date, priority: p.priority, estimate_minutes: p.estimate_minutes,
          ...(pointsIn(p) != null ? { points: pointsIn(p) } : {}),
          project_id: projects.find((x) => x.slug === p.project_slug)?.id ?? null, label_ids: label_ids.length ? label_ids : null, status_id,
        };
        issue = template ? (await api.templates.apply(template.id, fields)).issue : await api.issues.create(fields);
      } else {
        issue = await api.issues.createFromQuickadd(value);
      }
    } catch (err) {
      // Keep the text so a failed create can be fixed and retried.
      error = (err as { message?: string })?.message ?? String(err);
      return;
    }
    oncreated?.({
      issue,
      undo: async () => {
        await api.history.undo();
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

  // Focus is set here rather than with `autofocus`, which the a11y rules reject.
  $effect(() => {
    field?.focus();
  });
</script>

<Dialog bind:open onclosed={onclose} title="New issue">
  <div class="stage" aria-hidden="true"><Contour rings={7} size={180} /></div>
  <div class="field-wrap">
    <input
      bind:this={field}
      data-quickadd
      class="title"
      aria-label="Title, with quick-add tokens"
      placeholder={`Describe the issue.${hint}`}
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
              {#if c.key === 'priority'}<PriorityGlyph level={priorityGlyph(parsed?.priority ?? 0)} size={12} />{/if}
              {c.label}
            </button>
          {/snippet}
          {#if c.key === 'priority'}
            {#each PRIORITIES as name, level (name)}
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
          {:else if c.key === 'points'}
            {#each SCALES[scale] as s (s.label)}
              <MenuItem label={`${s.label} pts`} onSelect={() => setToken(POINTS_TOKEN, `${s.value}pt`)} />
            {/each}
            <MenuItem label="No points" onSelect={() => setToken(POINTS_TOKEN, '')} />
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
  .stage { position: absolute; right: calc(var(--s-7) * -1); top: calc(var(--s-7) * -1); color: var(--stone); opacity: 0.12; z-index: 0; pointer-events: none; }
  .field-wrap { position: relative; z-index: 1; }
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
  .chips { position: relative; z-index: 1; display: flex; flex-wrap: wrap; gap: var(--s-3); }
  .chip { display: inline-flex; align-items: center; gap: var(--s-3); height: var(--row-h-compact); padding: 0 var(--s-4); border: 1px solid var(--control); border-radius: var(--r-round); background: var(--raised); color: var(--ink); font-size: var(--fs-sm); font-weight: 600; transition: background var(--dur-1) var(--ease); }
  .chip:hover { background: var(--active); }
  .error { margin: 0; color: var(--signal); font-size: var(--fs-sm); font-weight: 500; }
  .hints { display: flex; align-items: center; gap: var(--s-5); margin-right: auto; color: var(--muted); font-size: var(--fs-xs); }
  .hints span { display: inline-flex; align-items: center; gap: var(--s-2); }
</style>

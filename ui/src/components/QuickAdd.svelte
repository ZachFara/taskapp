<script lang="ts">
  import { fieldOn } from '../lib/fields.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import type { Issue, IssueTemplate, NewIssue, ParsedQuickAdd, Project, Status } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import { defaultStatusId, projects, refreshAll, statuses } from '../lib/stores.svelte';
  import { viewContext } from '../lib/viewContext.svelte';
  import Button from '../ui/Button.svelte';
  import Contour from '../ui/Contour.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Menu from '../ui/Menu.svelte';
  import MenuItem from '../ui/MenuItem.svelte';
  import PriorityGlyph from '../ui/PriorityGlyph.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';
  import { humanDate, PRIORITIES, priorityGlyph } from './detail/format';
  import { mergeIssueDefaults, type IssueDefaults } from './filters/defaults';
  import { pointsOn, readEstimation, SCALES, type Estimation, type ScaleName } from './insights/estimation';
  import { byStatusOrder } from '../lib/statusOrder';

  // `project_slug`, `status_id` and `defaults` prefill from the screen that opened the dialog (a view, a column, a project).
  // `oncreated` hands the owner the new issue and an undo for the whole create, so it can show the toast.
  let {
    onclose,
    oncreated,
    project_slug = null,
    status_id = null,
    defaults = null,
  }: {
    onclose: () => void;
    oncreated?: (created: { issue: Issue; undo: () => Promise<void>; mode: 'close' | 'open' | 'keep' }) => void;
    project_slug?: string | null;
    status_id?: number | null;
    defaults?: IssueDefaults | null;
  } = $props();

  const DUE_WORDS = String.raw`(?:today|tomorrow|next week|(?:mon|tue|wed|thu|fri|sat|sun)\w*|\d{4}-\d{2}-\d{2}|in \d+ (?:day|days|week|weeks))`;

  let open = $state(true);
  // The prefill is the starting text only; the dialog owns the field after it opens.
  const prefill = untrack(() => (project_slug ? `+${project_slug} ` : ''));
  const initialDefaults = (): IssueDefaults =>
    mergeIssueDefaults(defaults, status_id !== null ? { status_id } : null);
  let activeDefaults = $state<IssueDefaults>(untrack(initialDefaults));
  let text = $state(prefill);
  let parsed = $state<ParsedQuickAdd | null>(null);
  let error = $state('');
  let templates = $state<IssueTemplate[]>([]);
  let labels = $state<{ id: number; name: string }[]>([]);
  let statusList = $state<Status[]>(statuses);
  let projectList = $state<Project[]>(projects);
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
  void api.statuses.list().then((s) => (statusList = s)).catch(() => {});
  void api.projects.list().then((p) => (projectList = p)).catch(() => {});

  // Typing `/` at the start opens the template list; the list narrows as the slug is typed.
  const slashQuery = $derived(/^\/(\S*)$/.exec(text)?.[1] ?? null);
  const matches = $derived(
    slashQuery === null ? [] : templates.filter((t) => t.slug.startsWith(slashQuery.toLowerCase())),
  );
  const template = $derived(templates.find((t) => text.startsWith(`/${t.slug} `) || text === `/${t.slug}`));

  const defaultStatus = $derived(statusList.find((s) => s.id === activeDefaults.status_id) ?? null);
  // The workspace's starting status is where the issue lands anyway, so it is not news: only a status the screen
  // chose (a column, a view) is worth a chip.
  const startingStatusId = $derived(
    defaultStatusId.value ?? [...statusList].sort(byStatusOrder)[0]?.id ?? null,
  );
  const defaultProject = $derived(projectList.find((p) => p.id === activeDefaults.project_id) ?? null);
  const defaultLabels = $derived(
    (activeDefaults.label_ids ?? [])
      .map((id) => labels.find((l) => l.id === id))
      .filter((l): l is { id: number; name: string } => !!l),
  );
  const effectivePriority = $derived(parsed?.priority ?? activeDefaults.priority ?? null);
  const effectiveDue = $derived(parsed?.due ?? activeDefaults.due ?? null);
  const effectiveStart = $derived(parsed?.start_date ?? activeDefaults.start_date ?? null);
  const effectiveEstimate = $derived(parsed?.estimate_minutes ?? activeDefaults.estimate_minutes ?? null);
  const effectivePoints = $derived(pointsIn(parsed) ?? activeDefaults.points ?? null);
  const effectiveProjectSlug = $derived(parsed?.project_slug ?? defaultProject?.slug ?? null);
  const effectiveLabelNames = $derived([
    ...new Set([
      ...defaultLabels.map((l) => l.name),
      ...(parsed?.label_names ?? []),
    ]),
  ]);
  const propEntries = $derived(Object.entries(activeDefaults.properties ?? {}));

  // Each chip is a Menu trigger: Tab reaches it in order, Enter or Space opens its menu, and the menu rewrites
  // that token (or the inherited view default), so the user always sees what will be set.
  // Only the tokens of fields that are switched on.
  const hint = $derived([['priority', 'p2'], ['project', '+project'], ['labels', '#label'], ['estimate', '~30m'], ['due', 'tomorrow']].flatMap(([f, t]) => (fieldOn(f as 'due') ? [t] : [])).map((t) => ` ${t}`).join(''));
  const chips = $derived(
    [
      template && { key: 'template', label: `/${template.slug}` },
      defaultStatus && defaultStatus.id !== startingStatusId && { key: 'status', label: defaultStatus.name },
      effectiveDue && { key: 'due', label: `Due ${humanDate(effectiveDue.Date ?? effectiveDue.DateTime!.utc.slice(0, 10))}` },
      effectiveStart && { key: 'start', label: `Start ${humanDate(effectiveStart)}` },
      effectivePriority != null && { key: 'priority', label: PRIORITIES[effectivePriority] ?? `P${effectivePriority}` },
      effectiveProjectSlug && { key: 'project', label: `+${effectiveProjectSlug}` },
      ...effectiveLabelNames.map((l) => ({ key: 'labels', label: `#${l}` })),
      effectiveEstimate != null && { key: 'estimate', label: `${effectiveEstimate}m` },
      effectivePoints != null && pointsOn(estimation) && { key: 'points', label: `${effectivePoints} pts` },
      ...propEntries.map(([k, v]) => ({ key: `prop:${k}`, label: `${k}: ${Array.isArray(v) ? v.join(', ') : String(v)}` })),
    ].filter((c): c is { key: string; label: string } => !!c),
  );

  function dropDefault(key: keyof IssueDefaults) {
    const next = { ...activeDefaults };
    delete next[key];
    activeDefaults = next;
  }

  // Replaces the token matching `pattern` with `token` (or drops it when `token` is empty). Chip menus use it.
  function setToken(pattern: RegExp, token: string, defaultKey?: keyof IssueDefaults) {
    if (defaultKey) dropDefault(defaultKey);
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

  const hasDefaults = (d: IssueDefaults) =>
    Object.entries(d).some(([, v]) =>
      v !== undefined &&
      v !== null &&
      (!Array.isArray(v) || v.length > 0) &&
      (typeof v !== 'object' || Object.keys(v).length > 0),
    );

  async function submit(mode: 'close' | 'open' | 'keep') {
    const value = text.trim();
    if (!value || (slashQuery !== null && !template)) return;
    let issue: Issue;
    try {
      if (hasDefaults(activeDefaults)) {
        // The quick-add grammar has no status/filter token for everything, so the parsed fields and view defaults
        // go into one write: a template apply or a create, either way one undo step.
        const rawText = template ? value.slice(template.slug.length + 1).trim() : value;
        const p = await api.issues.parseQuickadd(rawText);
        const found = await api.labels.list();
        let title = p.title;
        const parsedProject = p.project_slug === null ? undefined : projectList.find((x) => x.slug === p.project_slug);
        if (p.project_slug !== null && !parsedProject) title += ` +${p.project_slug}`;
        const parsedLabelIds: number[] = [];
        for (const name of p.label_names) {
          const hit = found.find((l) => l.name.toLowerCase() === name.toLowerCase());
          if (hit) parsedLabelIds.push(hit.id);
          else title += ` #${name}`;
        }
        const label_ids = [...new Set([...(activeDefaults.label_ids ?? []), ...parsedLabelIds])];
        const pts = pointsIn(p) ?? activeDefaults.points;
        const fields: NewIssue = {
          title,
          due: p.due ?? activeDefaults.due ?? null,
          start_date: p.start_date ?? activeDefaults.start_date ?? null,
          priority: p.priority ?? activeDefaults.priority ?? null,
          estimate_minutes: p.estimate_minutes ?? activeDefaults.estimate_minutes ?? null,
          ...(pts != null ? { points: pts } : {}),
          project_id: parsedProject?.id ?? activeDefaults.project_id ?? null,
          label_ids: label_ids.length ? label_ids : null,
          status_id: activeDefaults.status_id ?? null,
          ...(activeDefaults.cycle_id != null ? { cycle_id: activeDefaults.cycle_id } : {}),
          ...(activeDefaults.milestone_id != null ? { milestone_id: activeDefaults.milestone_id } : {}),
          ...(activeDefaults.parent_id != null ? { parent_id: activeDefaults.parent_id } : {}),
          ...(activeDefaults.properties && Object.keys(activeDefaults.properties).length
            ? { properties: activeDefaults.properties }
            : {}),
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
    await refreshAll(viewContext.viewDef ?? undefined).catch(() => {});
    oncreated?.({
      issue,
      mode,
      undo: async () => {
        await api.history.undo();
        void refreshAll(viewContext.viewDef ?? undefined).catch(() => {});
      },
    });
    if (mode === 'keep') {
      text = prefill;
      parsed = null;
      activeDefaults = initialDefaults();
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
      placeholder={`Title${hint}`}
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
              {#if c.key === 'status' && defaultStatus}<StatusGlyph category={defaultStatus.category} icon={defaultStatus.icon} color={defaultStatus.color} size={12} />{/if}
              {#if c.key === 'priority'}<PriorityGlyph level={priorityGlyph(effectivePriority ?? 0)} size={12} />{/if}
              {c.label}
            </button>
          {/snippet}
          {#if c.key === 'status'}
            {#each statusList as s (s.id)}
              <MenuItem label={s.name} onSelect={() => (activeDefaults = { ...activeDefaults, status_id: s.id })} />
            {/each}
            <MenuItem label="Default status" onSelect={() => dropDefault('status_id')} />
          {:else if c.key === 'priority'}
            {#each PRIORITIES as name, level (name)}
              <MenuItem label={name} onSelect={() => setToken(/(^|\s)p[0-4](?=\s|$)/i, level ? `p${level}` : '', 'priority')} />
            {/each}
          {:else if c.key === 'project'}
            {#each projectList as p (p.slug)}
              <MenuItem label={p.name} icon="project" onSelect={() => setToken(/(^|\s)\+[\w-]+/, `+${p.slug}`, 'project_id')} />
            {/each}
            <MenuItem label="No project" onSelect={() => setToken(/(^|\s)\+[\w-]+/, '', 'project_id')} />
          {:else if c.key === 'due'}
            <MenuItem label="Today" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'today', 'due')} />
            <MenuItem label="Tomorrow" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'tomorrow', 'due')} />
            <MenuItem label="Next week" icon="calendar" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), 'next week', 'due')} />
            <MenuItem label="No due date" onSelect={() => setToken(new RegExp(`(^|\\s)(by |due )?${DUE_WORDS}(?=\\s|$)`, 'i'), '', 'due')} />
          {:else if c.key === 'start'}
            <MenuItem label="Start today" icon="clock" onSelect={() => setToken(/(^|\s)start\s+\S+(\s+\S+)?/i, 'start today', 'start_date')} />
            <MenuItem label="No start date" onSelect={() => setToken(/(^|\s)start\s+\S+(\s+\S+)?/i, '', 'start_date')} />
          {:else if c.key === 'labels'}
            {#each labels as l (l.name)}
              <MenuItem label={l.name} icon="tag" onSelect={() => {
                const inText = text.toLowerCase().includes(`#${l.name.toLowerCase()}`);
                const inDefaults = (activeDefaults.label_ids ?? []).includes(l.id);
                if (inDefaults) {
                  activeDefaults = { ...activeDefaults, label_ids: (activeDefaults.label_ids ?? []).filter((id) => id !== l.id) };
                } else {
                  setToken(new RegExp(`(^|\\s)[#@]${l.name}(?=\\s|$)`, 'i'), inText ? '' : `#${l.name}`);
                }
              }} />
            {/each}
          {:else if c.key === 'estimate'}
            <MenuItem label="30 minutes" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~30m', 'estimate_minutes')} />
            <MenuItem label="2 hours" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~2h', 'estimate_minutes')} />
            <MenuItem label="1 day" icon="clock" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '~1d', 'estimate_minutes')} />
            <MenuItem label="No estimate" onSelect={() => setToken(/(^|\s)~\w+(?=\s|$)/, '', 'estimate_minutes')} />
          {:else if c.key === 'points'}
            {#each SCALES[scale] as s (s.label)}
              <MenuItem label={`${s.label} pts`} onSelect={() => setToken(POINTS_TOKEN, `${s.value}pt`, 'points')} />
            {/each}
            <MenuItem label="No points" onSelect={() => setToken(POINTS_TOKEN, '', 'points')} />
          {:else if c.key === 'template'}
            {#each templates as t (t.id)}
              <MenuItem label={t.name} icon="layers" onSelect={() => pickTemplate(t)} />
            {/each}
          {:else if c.key.startsWith('prop:')}
            {@const propKey = c.key.slice(5)}
            <MenuItem label={`Clear ${propKey}`} onSelect={() => {
              const next = { ...(activeDefaults.properties ?? {}) };
              delete next[propKey];
              activeDefaults = { ...activeDefaults, properties: Object.keys(next).length ? next : undefined };
            }} />
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
  .stage { position: absolute; right: calc(var(--s-7) * -1); top: calc(var(--s-7) * -1); color: var(--stone); opacity: 0.08; z-index: 0; pointer-events: none; }
  .field-wrap { position: relative; z-index: 1; }
  .title {
    width: 100%;
    height: calc(var(--ctl-h) + var(--s-6));
    padding: 0 var(--s-2);
    border: 0;
    border-bottom: 1px solid var(--control);
    background: transparent;
    color: var(--ink);
    font-size: var(--fs-lg);
    font-weight: 600;
    letter-spacing: -0.01em;
    transition: border-color var(--dur-1) var(--ease);
  }
  .title::placeholder { color: var(--muted); font-weight: 500; }
  .title:focus-visible { outline: none; border-color: var(--focus); box-shadow: 0 1px 0 var(--focus); }
  .templates { position: absolute; z-index: var(--z-popover); top: 100%; left: 0; right: 0; margin: var(--s-2) 0 0; padding: var(--s-2); list-style: none; background: var(--sheet); border: 1px solid var(--control); border-radius: var(--r-pop); box-shadow: var(--shadow-float); }
  .option { display: flex; gap: var(--s-4); width: 100%; height: var(--ctl-h); align-items: center; padding: 0 var(--s-4); border: 0; border-radius: var(--r-sm); background: transparent; color: var(--ink); text-align: left; }
  .option[data-active] { background: var(--active); }
  .slug { font-weight: 700; color: var(--accent); }
  .name { color: var(--muted); }
  .chips { position: relative; z-index: 1; display: flex; flex-wrap: wrap; gap: var(--s-3); padding: var(--s-3) var(--s-2) 0; }
  .chip { display: inline-flex; align-items: center; gap: var(--s-3); height: var(--row-h-compact); padding: 0 var(--s-4); border: 1px solid var(--control); border-radius: var(--r-round); background: var(--raised); color: var(--ink); font-size: var(--fs-sm); font-weight: 600; transition: background var(--dur-1) var(--ease); }
  .chip:hover { background: var(--active); }
  .error { margin: 0; color: var(--signal); font-size: var(--fs-sm); font-weight: 500; }
  .hints { display: flex; align-items: center; gap: var(--s-5); margin-right: auto; color: var(--muted); font-size: var(--fs-xs); }
  .hints span { display: inline-flex; align-items: center; gap: var(--s-2); }
</style>

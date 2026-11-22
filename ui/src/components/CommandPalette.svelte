<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { tick, untrack } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Cycle, Issue, Label } from '../lib/bindings';
  import { listCommands, shortcutFor, type Command, type CommandPage, type Ctx } from '../lib/commands';
  import { rank } from '../lib/commands/match';
  import { selection } from '../lib/selection.svelte';
  import { issues, projects, statuses, views } from '../lib/stores.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import Icon from '../ui/Icon.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';

  let { onclose, oncreate }: { onclose: () => void; oncreate: () => void } = $props();

  // A page may also be multi-pick (stays open, items show a check) or free-input (`parse` turns the typed text into
  // one preview row); the registry type does not carry these yet, so they are read here as optional extras.
  type PageItem = CommandPage['items'][number] & { checked?: () => boolean };
  type Page = Omit<CommandPage, 'items'> & {
    items: PageItem[];
    multi?: boolean;
    parse?: (text: string) => { label: string; run: () => void | Promise<void> } | null;
  };
  type Row = {
    id: string;
    label: string;
    group: string;
    keywords?: string[];
    icon?: string;
    entity?: { icon: string | null | undefined; color: string | null | undefined };
    category?: string;
    keys?: string | string[];
    more?: boolean;
    checked?: boolean;
    run: () => void | Promise<void>;
  };

  let query = $state('');
  let active = $state(0);
  let stack = $state<{ title: string; cmd: Command; page: Page }[]>([]);
  let found = $state<Issue[]>([]);
  let cycles = $state<Cycle[]>([]);
  let labels = $state<Label[]>([]);
  let inputEl = $state<HTMLInputElement>();
  let seq = 0;

  // Recently run ids, newest first; kept for the session.
  const recent = (globalThis as { __cairnRecent?: string[] }).__cairnRecent ?? ((globalThis as { __cairnRecent?: string[] }).__cairnRecent = []);

  const issueByKey = () => /^\/issue\/([^/]+)/.exec(location.pathname)?.[1];
  const ctx = (): Ctx => {
    const open = issues.find((i) => i.key === issueByKey());
    return {
      route: location.pathname,
      selection: selection.selected.length ? [...selection.selected] : selection.focused !== null ? [selection.focused] : [],
      focusedIssueId: selection.focused,
      openIssueId: open?.id ?? null,
      api,
      navigate,
    };
  };
  const c0 = ctx();
  const contextTitle = c0.selection.length > 1
    ? `${c0.selection.length} issues selected`
    : c0.selection.length === 1 || c0.openIssueId !== null
      ? `Actions on ${issues.find((i) => i.id === (c0.selection[0] ?? c0.openIssueId))?.key ?? 'issue'}`
      : 'Actions';

  void Promise.all([api.cycles.list(), api.labels.list()]).then(([c, l]) => { cycles = c; labels = l; }).catch(() => {});

  const nav = (path: string) => () => navigate(path);
  const SETTINGS = ['general', 'statuses', 'properties', 'projects', 'cycles', 'labels', 'reminders', 'calendar', 'export', 'backups'];

  const commandRows = (): Row[] =>
    listCommands(ctx()).map((c) => ({
      id: `cmd:${c.id}`,
      label: c.page ? `${c.label}…` : c.label,
      keywords: [c.label, ...(c.keywords ?? [])],
      group: c.when ? contextTitle : c.group,
      icon: c.icon,
      keys: shortcutFor(c.id),
      more: !!c.page,
      run: () => {
        if (c.page) {
          const page = c.page(ctx()) as Page;
          stack.push({ title: page.title, cmd: c, page });
          query = '';
          active = 0;
          return;
        }
        // The create screen is mounted by the shell, which hands us this callback.
        if (c.id === 'issue.create') return oncreate();
        return c.run?.(ctx());
      },
    }));

  const targetRows = $derived.by((): Row[] => [
    ...found.map((i) => ({
      id: `issue:${i.id}`, label: `${i.key} ${i.title}`, group: 'Issues', category: statuses.find((s) => s.id === i.status_id)?.category,
      run: nav(`/issue/${i.key}`),
    })),
    ...views.map((v) => ({ id: `view:${v.id}`, label: v.name, group: 'Views', entity: { icon: v.icon ?? 'icon:view', color: v.color }, icon: v.icon ? undefined : 'layers', run: nav(`/view/${v.id}`) })),
    ...projects.map((p) => ({ id: `project:${p.id}`, label: p.name, group: 'Projects', icon: 'project', run: nav(`/project/${p.slug}`) })),
    ...cycles.map((c) => ({ id: `cycle:${c.id}`, label: c.name ?? `Cycle ${c.number}`, group: 'Cycles', icon: 'cycle', run: nav('/cycles') })),
    ...labels.map((l) => ({ id: `label:${l.id}`, label: l.name, group: 'Labels', entity: { icon: l.icon ?? 'icon:tag', color: l.color }, run: nav('/settings/labels') })),
    ...SETTINGS.map((s) => ({ id: `settings:${s}`, label: `Settings: ${s[0].toUpperCase()}${s.slice(1)}`, group: 'Settings', icon: 'settings', run: nav(`/settings/${s}`) })),
  ]);

  const parsed = $derived.by(() => {
    const top = stack.at(-1)?.page;
    const q = query.trim();
    return top?.parse && q ? top.parse(q) : null;
  });

  // Groups in display order: context first, then the user's commands, then things to jump to.
  const groups = $derived.by(() => {
    const top = stack.at(-1);
    let rows: Row[];
    if (top) {
      rows = top.page.items.map((it) => ({
        id: `page:${it.id}`, label: it.label, keywords: it.keywords, group: top.title, icon: it.icon,
        checked: it.checked ? it.checked() : undefined,
        run: async () => {
          await it.run();
          if (top.page.multi) {
            stack[stack.length - 1] = { ...top, page: top.cmd.page!(ctx()) as Page };
            await tick();
          } else onclose();
        },
      }));
      if (parsed) rows = [{ id: 'parsed', label: parsed.label, group: 'Preview', icon: 'check', run: async () => { await parsed.run(); onclose(); } }, ...rows];
    } else rows = [...commandRows(), ...targetRows];
    const q = query.trim();
    const ranked = parsed || !q ? rows : rank(q, rows.map((r) => ({ ...r, keywords: [r.label, ...(r.keywords ?? [])] })), recent) as Row[];
    const out: { name: string; rows: Row[] }[] = [];
    if (!top && !q) {
      const recents = recent.flatMap((id) => rows.filter((r) => r.id === id)).slice(0, 4);
      if (recents.length) out.push({ name: 'Recent', rows: recents });
    }
    for (const r of ranked) {
      const g = out.find((x) => x.name === r.group) ?? (out.push({ name: r.group, rows: [] }), out.at(-1)!);
      g.rows.push(r);
    }
    return out;
  });
  const flat = $derived(groups.flatMap((g) => g.rows));
  let index = 0;

  async function search(q: string) {
    const mine = ++seq;
    if (!q || stack.length) return void (found = []);
    const rows = await api.search.search(q, 8).catch(() => [] as Issue[]);
    // A key such as ABC-3 is matched exactly; full-text search does not index keys.
    if (/^[a-z]+-\d+$/i.test(q)) {
      const hit = await api.issues.getByKey(q.toUpperCase()).catch(() => null);
      if (hit && !rows.some((r) => r.id === hit.id)) rows.unshift(hit);
    }
    if (mine === seq) found = rows;
  }

  $effect(() => {
    const q = query.trim();
    untrack(() => void search(q));
  });
  $effect(() => {
    void query;
    void stack.length;
    active = 0;
  });
  $effect(() => {
    document.getElementById(`cp-${active}`)?.scrollIntoView?.({ block: 'nearest' });
  });

  async function choose(row: Row | undefined) {
    if (!row) return;
    const isPage = row.id.startsWith('page:') || row.id === 'parsed';
    if (!isPage) {
      recent.splice(0, recent.length, row.id, ...recent.filter((x) => x !== row.id).slice(0, 7));
    }
    // A command that opens a page keeps the menu; a plain command closes it first so focus returns to the screen.
    if (!isPage && !(row.more)) onclose();
    await row.run();
  }

  const back = () => (stack.length ? (stack.pop(), (query = '')) : onclose());

  function onkeydown(e: KeyboardEvent) {
    const n = flat.length;
    const ctrl = e.ctrlKey && !e.metaKey && !e.altKey;
    const step = e.key === 'ArrowDown' || (ctrl && (e.key === 'n' || e.key === 'j')) ? 1
      : e.key === 'ArrowUp' || (ctrl && (e.key === 'p' || e.key === 'k')) ? -1 : 0;
    if (step) {
      e.preventDefault();
      if (n) active = (active + step + n) % n;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      void choose(flat[active]);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const r = flat[active];
      if (r) query = r.label.replace(/…$/, '');
    } else if (e.key === 'Backspace' && !query && stack.length) {
      e.preventDefault();
      back();
    }
  }
  // Rows are numbered in display order for aria-activedescendant.
  const numbered = $derived((index = 0, groups.map((g) => ({ ...g, rows: g.rows.map((r) => ({ r, n: index++ })) }))));
</script>

<Dialog.Root open onOpenChange={(o) => !o && onclose()}>
  <Dialog.Portal>
    <Dialog.Overlay class="cp-overlay" />
    <Dialog.Content
      class="cp"
      aria-label="Command palette"
      aria-describedby={undefined}
      onOpenAutoFocus={(e) => { e.preventDefault(); inputEl?.focus(); }}
      onEscapeKeydown={(e) => { e.preventDefault(); back(); }}
    >
      <Dialog.Title class="cp-sr">Command menu</Dialog.Title>
      <div class="field">
        <Icon name="search" />
        {#each stack as s (s.title)}<span class="crumb">{s.title}</span>{/each}
        <input
          bind:this={inputEl}
          bind:value={query}
          {onkeydown}
          role="combobox"
          aria-expanded="true"
          aria-controls="cp-list"
          aria-activedescendant={flat.length ? `cp-${active}` : undefined}
          aria-label={stack.length ? stack.at(-1)!.title : 'Search commands, issues, views and projects'}
          placeholder={stack.length ? 'Filter…' : 'Type a command, issue, view or project'}
          autocomplete="off"
          spellcheck="false"
        />
        <Kbd keys="Esc" />
      </div>
      <div class="list" id="cp-list" role="listbox" aria-multiselectable={stack.at(-1)?.page.multi || undefined}>
        {#each numbered as g (g.name)}
          <div role="group" aria-label={g.name}>
            <h4 class:ctx={g.name === contextTitle}>{g.name}</h4>
            {#each g.rows as { r, n } (r.id)}
              <!-- Mouse down must not steal focus from the field. -->
              <div
                id="cp-{n}"
                class="row"
                class:cur={n === active}
                role="option"
                tabindex="-1"
                aria-selected={n === active}
                aria-checked={r.checked === undefined ? undefined : r.checked}
                onmousemove={() => (active = n)}
                onmousedown={(e) => e.preventDefault()}
                onclick={() => choose(r)}
                onkeydown={() => {}}
              >
                <span class="ico">
                  {#if r.category}<StatusGlyph category={r.category as never} />
                  {:else if r.entity}<EntityIcon icon={r.entity.icon} color={r.entity.color} />
                  {:else if r.icon}<Icon name={r.icon} />{/if}
                </span>
                <span class="lab">{r.label}</span>
                <span class="sp"></span>
                {#if r.checked}<span class="tick"><Icon name="check" /></span>{/if}
                {#if r.more}<span class="more"><Icon name="chevron-right" /></span>{/if}
                {#if r.keys}<Kbd keys={r.keys} />{/if}
              </div>
            {/each}
          </div>
        {:else}
          <p class="empty">No matches</p>
        {/each}
      </div>
      <div class="foot">
        <span><Kbd keys="↑" /><Kbd keys="↓" /> move</span>
        <span><Kbd keys="↵" /> {stack.at(-1)?.page.multi ? 'toggle' : 'run'}</span>
        <span class="sp"></span>
        <span>{stack.length ? 'Backspace goes back' : 'Tab completes'}</span>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.cp-overlay) { position: fixed; inset: 0; z-index: var(--z-dialog); background: var(--overlay); animation: cp-fade var(--dur-2) var(--ease); }
  :global(.cp) {
    position: fixed; z-index: var(--z-dialog); top: 14%; left: 50%; transform: translateX(-50%);
    width: min(640px, calc(100% - var(--s-9)));
    display: grid; grid-template-rows: auto minmax(0, 1fr) auto; max-height: 68vh;
    background: var(--sheet); color: var(--ink); border: 1px solid var(--control); border-radius: 0;
    box-shadow: var(--shadow-float); font-family: var(--font); animation: cp-fade var(--dur-2) var(--ease);
  }
  :global(.cp-sr) { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  .field { display: flex; align-items: center; gap: var(--s-4); padding: var(--s-5) var(--s-6); border-bottom: 1px solid var(--control); color: var(--muted); }
  .crumb { flex: none; padding: var(--s-1) var(--s-4); background: var(--active); color: var(--ink); font-size: var(--fs-sm); font-weight: 700; border-left: 3px solid var(--accent); }
  input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--ink); font: 500 var(--fs-lg) / var(--lh-tight) var(--font); }
  input::placeholder { color: var(--muted); }
  .list { overflow-y: auto; padding-bottom: var(--s-2); }
  h4 { margin: 0; padding: var(--s-2) var(--s-6); background: var(--band-1); color: var(--band-ink); font-size: var(--fs-xs); font-weight: 800; letter-spacing: 0.06em; text-transform: uppercase; }
  h4.ctx { background: var(--band-3); }
  .row { display: flex; align-items: center; gap: var(--s-5); height: var(--row-h); padding: 0 var(--s-6); border-left: 3px solid transparent; font-size: var(--fs-md); cursor: default; }
  .row.cur { background: var(--selection); border-left-color: var(--accent); }
  .ico { display: grid; place-items: center; width: var(--icon); color: var(--muted); }
  .lab { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .sp { flex: 1; }
  .tick { color: var(--accent); }
  .more { color: var(--muted); }
  .empty { margin: 0; padding: var(--s-8) var(--s-6); text-align: center; color: var(--muted); }
  .foot { display: flex; align-items: center; gap: var(--s-7); padding: var(--s-3) var(--s-6); border-top: 1px solid var(--border); background: var(--raised); color: var(--muted); font-size: var(--fs-xs); }
  .foot span { display: inline-flex; align-items: center; gap: var(--s-2); }
  @keyframes cp-fade { from { opacity: 0; } }
</style>

<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { getContext, onMount, tick, untrack } from 'svelte';
  import { navigate } from '../app/routes';
  import type { Status } from '../lib/bindings';
  import { api } from '../lib/api';
  import type { Cycle, Issue, Label } from '../lib/bindings';
  import { listCommands, sectionOf, shortcutFor, type Command, type CommandPage, type Ctx } from '../lib/commands';
  import { rank } from '../lib/commands/match';
  import { replay } from '../lib/motion.svelte';
  import { selection } from '../lib/selection.svelte';
  import { issues, projects, statuses, views } from '../lib/stores.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import Icon from '../ui/Icon.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import StatusGlyph from '../ui/StatusGlyph.svelte';

  let { onclose, oncreate, page: startPage, issues: startIssues }: { onclose: () => void; oncreate: () => void; page?: string; issues?: boolean } = $props();

  // A page may also be multi-pick (stays open, items show a check) or free-input (`parse` turns the typed text into
  // one preview row); the registry type does not carry these yet, so they are read here as optional extras.
  // `checked` is a boolean in the registry type; a function is tolerated too. A page may arrive as a Promise or
  // carry `load()` (items then come from its result), and a parse result may carry its own `run`.
  type PageItem = Omit<CommandPage['items'][number], 'checked'> & { checked?: boolean | (() => boolean) };
  type Page = Omit<CommandPage, 'items' | 'parse'> & {
    items: PageItem[];
    multi?: boolean;
    parse?: (text: string) => { label: string; value?: unknown; run?: () => void | Promise<void> } | null;
    load?: () => PageItem[] | void | Promise<PageItem[] | void>;
  };
  type Row = {
    id: string;
    label: string;
    group: string;
    keywords?: string[];
    icon?: string;
    entity?: { icon: string | null | undefined; color: string | null | undefined };
    status?: Status;
    keys?: string | string[];
    more?: boolean;
    checked?: boolean;
    /** Set on rows that only navigate, so a recent one can be rebuilt without searching for it again. */
    path?: string;
    run: () => void | Promise<void>;
  };

  // `open` ends the menu's life with its exit animation; `onclose` tells the app once that has played.
  let open = $state(true);
  // Focus goes back at once, so keys typed during the exit animation reach the screen, not the leaving field.
  const prevFocus = document.activeElement as HTMLElement | null;
  const close = () => {
    open = false;
    inputEl?.blur();
    prevFocus?.isConnected && prevFocus.focus?.();
  };
  let query = $state('');
  let active = $state(0);
  // Search issues only (opened by `/` or the "Search issues…" row): no commands, no views, no projects.
  let issuesOnly = $state(startIssues ?? false);
  let stack = $state<{ title: string; cmd: Command; page: Page; loading?: boolean }[]>([]);
  let found = $state<Issue[]>([]);
  let cycles = $state<Cycle[]>([]);
  let labels = $state<Label[]>([]);
  let inputEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLElement>();
  // Rows stagger in once, as the menu opens; typing must not replay it. A page change slides the list sideways.
  let opening = $state(true);
  onMount(() => { const t = setTimeout(() => (opening = false), 400); return () => clearTimeout(t); });
  let depth = 0;
  $effect(() => {
    const d = stack.length;
    if (listEl && d !== depth) {
      listEl.style.setProperty('--cx-px', d > depth ? '24px' : '-24px');
      replay(listEl, 'cx-page');
    }
    depth = d;
  });
  let seq = 0;

  // Recently chosen rows, newest first, kept across reloads. A row that only navigates keeps its path and label, since
  // an issue is not in the root list until it is searched for again.
  type Recent = { id: string; label: string; path?: string; icon?: string };
  const KEY = 'cairn.recent';
  const recent: Recent[] = (() => {
    try {
      const raw = JSON.parse(localStorage.getItem(KEY) ?? '[]');
      return Array.isArray(raw) ? raw.filter((r) => typeof r?.id === 'string' && typeof r?.label === 'string') : [];
    } catch {
      return [];
    }
  })();
  let recentIssues = $state<Record<string, Issue>>({});
  onMount(() => {
    let mounted = true;
    for (const r of recent) {
      const match = /^issue:(\d+)$/.exec(r.id);
      if (!match) continue;
      void api.issues.get(Number(match[1])).then((issue) => {
        if (mounted) recentIssues[r.id] = issue;
      }).catch(() => {});
    }
    return () => { mounted = false; };
  });
  const remember = (row: Row) => {
    recent.splice(0, recent.length, { id: row.id, label: row.label, path: row.path, icon: row.icon }, ...recent.filter((x) => x.id !== row.id).slice(0, 7));
    try {
      localStorage.setItem(KEY, JSON.stringify(recent));
    } catch {
      /* private mode or a full store: recents last for this session only */
    }
  };

  const issueByKey = () => /^\/issue\/([^/]+)/.exec(location.pathname)?.[1];
  // The app's live context knows the layout and the open panel; a bare mount (tests) falls back to reading the page.
  const live = getContext<(() => Ctx) | undefined>('cmdctx');
  const ctx = (): Ctx => {
    if (live) {
      const c = live();
      const one = c.selection.length ? c.selection : c.focusedIssueId !== null ? [c.focusedIssueId] : [];
      return { ...c, selection: [...one] };
    }
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

  // Sync pages show at once; an async page or one with `load()` shows a skeleton row until its items are in.
  async function openPage(c: Command) {
    const made = c.page!(ctx()) as Page | Promise<Page>;
    // pageOf hands back the page at once with its items still loading, so a Promise inside counts as loading too.
    const sync = !('then' in made) && !((made as Page).items instanceof Promise);
    const first = sync ? (made as Page) : null;
    const entry = { title: first?.title ?? c.label, cmd: c, page: first ?? ({ title: c.label, items: [] } as Page), loading: !sync || !!first?.load };
    stack.push(entry);
    const at = stack.length - 1;
    const page = sync ? (made as Page) : await (made as Promise<Page>);
    const items = page.load ? await page.load() : page.items instanceof Promise ? await page.items : undefined;
    if (stack[at]?.cmd.id !== c.id) return;
    stack[at] = { title: page.title, cmd: c, page: Array.isArray(items) ? { ...page, items } : page, loading: false };
  }

  // A key on a page command (`s`, `d`) opens the menu straight at that page.
  onMount(() => {
    const c = startPage ? listCommands(ctx(), { all: true }).find((x) => x.id === startPage) : undefined;
    if (c?.page) void openPage(c);
  });

  const commandRows = (): Row[] =>
    listCommands(ctx()).map((c) => ({
      id: `cmd:${c.id}`,
      label: c.page ? `${c.label}…` : c.label,
      keywords: [c.label, ...(c.keywords ?? [])],
      group: sectionOf(c) === 'Issue' ? contextTitle : sectionOf(c),
      icon: c.icon,
      keys: shortcutFor(c.id),
      more: !!c.page || c.id === 'nav.search',
      run: () => {
        if (c.id === 'nav.search') {
          query = '';
          issuesOnly = true;
          return;
        }
        if (c.page) {
          query = '';
          active = 0;
          return void openPage(c);
        }
        // The create screen is mounted by the shell, which hands us this callback.
        if (c.id === 'issue.create') return oncreate();
        return c.run?.(ctx());
      },
    }));

  const targetRows = $derived.by((): Row[] => [
    ...found.map((i) => ({
      id: `issue:${i.id}`, label: `${i.key} ${i.title}`, group: 'Issues', status: statuses.find((s) => s.id === i.status_id),
      path: `/issue/${i.key}`, run: nav(`/issue/${i.key}`),
    })),
    ...views.map((v) => ({ id: `view:${v.id}`, label: v.name, group: 'Views', entity: { icon: v.icon ?? 'icon:view', color: v.color }, icon: v.icon ? undefined : 'layers', path: `/view/${v.id}`, run: nav(`/view/${v.id}`) })),
    ...projects.map((p) => ({ id: `project:${p.id}`, label: p.name, group: 'Projects', icon: 'project', path: `/project/${p.slug}`, run: nav(`/project/${p.slug}`) })),
    ...cycles.map((c) => ({ id: `cycle:${c.id}`, label: c.name ?? `Cycle ${c.number}`, group: 'Cycles', icon: 'cycle', path: '/cycles', run: nav('/cycles') })),
    ...labels.map((l) => ({ id: `label:${l.id}`, label: l.name, group: 'Labels', entity: { icon: l.icon ?? 'icon:tag', color: l.color }, path: '/settings/labels', run: nav('/settings/labels') })),
  ]);

  const parsed = $derived.by(() => {
    const top = stack.at(-1)?.page;
    const q = query.trim();
    return top?.parse && q ? top.parse(q) : null;
  });

  const isIssue = (r: Row) => r.id.startsWith('issue:');
  const issueRows = $derived(targetRows.filter(isIssue));

  // Groups in display order: matching issues first, then context, the user's commands, then things to jump to.
  const groups = $derived.by(() => {
    const top = stack.at(-1);
    let rows: Row[];
    if (top) {
      rows = top.page.items.map((it) => ({
        id: `page:${it.id}`, label: it.label, keywords: it.keywords, group: top.title, icon: it.icon,
        checked: typeof it.checked === 'function' ? it.checked() : it.checked,
        run: async () => {
          await it.run();
          if (top.page.multi) {
            const again = (await Promise.resolve(top.cmd.page!(ctx()))) as Page;
            stack[stack.length - 1] = { ...top, page: { ...again, items: await again.items } };
            await tick();
          } else close();
        },
      }));
      if (parsed) rows = [{ id: 'parsed', label: parsed.label, group: 'Preview', icon: 'check', run: async () => { await parsed.run?.(); close(); } }, ...rows];
    } else rows = issuesOnly ? issueRows : [...commandRows(), ...targetRows];
    const q = query.trim();
    // Issues come from the backend already matched, so they are listed as found and never re-ranked by letters.
    const others = rank(q, rows.filter((r) => !isIssue(r)).map((r) => ({ ...r, keywords: [r.label, ...(r.keywords ?? [])] })), recent.map((r) => r.id)) as Row[];
    const ranked = parsed || !q || issuesOnly ? rows : [...rows.filter(isIssue), ...others];
    const out: { name: string; rows: Row[] }[] = [];
    if (!top && !q && !issuesOnly) {
      const recents = recent
        .flatMap((r): Row[] => {
          const live = rows.find((x) => x.id === r.id);
          const issue = recentIssues[r.id];
          return live ? [live] : r.path ? [{ id: r.id, label: r.label, icon: r.icon, status: statuses.find((s) => s.id === issue?.status_id), group: 'Recent', path: r.path, run: nav(r.path) }] : [];
        })
        .slice(0, 5);
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
      remember(row);
    }
    // A command that opens a page keeps the menu; a plain command closes it first so focus returns to the screen.
    if (!isPage && !(row.more)) close();
    await row.run();
  }

  const back = () => (stack.length ? (stack.pop(), (query = '')) : close());

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

<Dialog.Root bind:open onOpenChangeComplete={(o) => !o && onclose()}>
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
          aria-label={stack.length ? stack.at(-1)!.title : issuesOnly ? 'Search issues' : 'Search commands, issues, views and projects'}
          placeholder={stack.length ? 'Filter…' : issuesOnly ? 'Search issues…' : 'Type a command, issue, view or project'}
          autocomplete="off"
          spellcheck="false"
        />
        <Kbd keys="Esc" />
      </div>
      <div class="list" class:opening bind:this={listEl} id="cp-list" role="listbox" aria-multiselectable={stack.at(-1)?.page.multi || undefined}>
        {#each numbered as g (g.name)}
          <div role="group" aria-label={g.name}>
            <h4 class:ctx={g.name === contextTitle}>{g.name}</h4>
            {#each g.rows as { r, n } (r.id)}
              <!-- Mouse down must not steal focus from the field. -->
              <div
                id="cp-{n}"
                style:--i={n}
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
                  {#if r.status}<StatusGlyph category={r.status.category} icon={r.status.icon} color={r.status.color} />
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
          {#if stack.at(-1)?.loading}
            <div class="skel" aria-busy="true"><Skeleton height="var(--row-h)" /></div>
          {:else}<p class="empty">{issuesOnly && !query.trim() ? 'Type an issue title, word or key' : 'No matches'}</p>{/if}
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
  :global(.cp-overlay) { position: fixed; inset: 0; z-index: var(--z-dialog); background: var(--overlay); }
  :global(.cp) {
    position: fixed; z-index: var(--z-dialog); top: 14%; left: 50%; transform: translateX(-50%);
    width: min(640px, calc(100% - var(--s-9)));
    display: grid; grid-template-rows: auto minmax(0, 1fr) auto; max-height: 68vh;
    background: var(--sheet); color: var(--ink); border: 1px solid var(--control); border-radius: 0;
    box-shadow: var(--shadow-float); font-family: var(--font);
  }
  :global(.cp-sr) { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
  .field { display: flex; align-items: center; gap: var(--s-4); padding: var(--s-5) var(--s-6); border-bottom: 1px solid var(--control); color: var(--muted); }
  .crumb { flex: none; padding: var(--s-1) var(--s-4); background: var(--active); color: var(--ink); font-size: var(--fs-sm); font-weight: 700; border-left: 3px solid var(--accent); }
  input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--ink); font: 500 var(--fs-lg) / var(--lh-tight) var(--font); }
  input::placeholder { color: var(--muted); }
  .list { overflow-y: auto; padding-bottom: var(--s-2); }
  h4 { margin: 0; padding: var(--s-2) var(--s-6); background: var(--band-1); color: var(--band-ink); font-size: var(--fs-xs); font-weight: 800; letter-spacing: 0.06em; text-transform: uppercase; }
  h4.ctx { background: var(--band-3); }
  .row { display: flex; align-items: center; gap: var(--s-5); height: var(--row-h); padding: 0 var(--s-6); border-left: 3px solid transparent; font-size: var(--fs-md); cursor: default; transition: background var(--dur-1) var(--ease); }
  .opening .row { animation: cx-row-in var(--dur-4) var(--ease) backwards; animation-delay: calc(min(var(--i), 8) * var(--dur-stagger)); }
  .row.cur { background: var(--selection); border-left-color: var(--accent); }
  .ico { display: grid; place-items: center; width: var(--icon); color: var(--muted); }
  .lab { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .sp { flex: 1; }
  .tick { color: var(--accent); }
  .more { color: var(--muted); }
  .skel { padding: var(--s-2) var(--s-6); }
  .empty { margin: 0; padding: var(--s-8) var(--s-6); text-align: center; color: var(--muted); }
  .foot { display: flex; align-items: center; gap: var(--s-7); padding: var(--s-3) var(--s-6); border-top: 1px solid var(--border); background: var(--raised); color: var(--muted); font-size: var(--fs-xs); }
  .foot span { display: inline-flex; align-items: center; gap: var(--s-2); }
</style>

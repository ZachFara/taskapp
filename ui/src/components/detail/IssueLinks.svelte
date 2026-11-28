<script lang="ts">
  import { api } from '../../lib/api';
  import { isCairnLink, openCairnLink } from '../../lib/deepLink';
  import type { Issue, Link, LinkKind } from '../../lib/bindings';
  import { openExternal } from '../../lib/openExternal';
  import Button from '../../ui/Button.svelte';
  import Collapse from '../../ui/Collapse.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import SectionHeading from './SectionHeading.svelte';

  // The links of the open issue: where the fix, the PR or a doc lives. A Cairn link opens in the app, any other
  // goes through the link guard (openExternal), which refuses what it does not allow.
  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  const KINDS: { value: LinkKind; label: string }[] = [
    { value: 'solution', label: 'Solution' }, { value: 'reference', label: 'Reference' }, { value: 'pr', label: 'PR' },
    { value: 'commit', label: 'Commit' }, { value: 'doc', label: 'Doc' }, { value: 'other', label: 'Other' },
  ];
  const label = (k: LinkKind) => KINDS.find((x) => x.value === k)?.label ?? k;

  let rows = $state<Link[]>([]);
  let collapsed = $state(false);
  let url = $state('');
  let title = $state('');
  let kind = $state<string>('reference');
  let refused = $state('');
  let sec = $state<HTMLElement>();

  $effect(() => {
    rev;
    api.links.list(issue.id).then((l) => (rows = l), () => (rows = []));
  });

  async function open(l: Link) {
    refused = '';
    const ok = isCairnLink(l.url) ? await openCairnLink(l.url, api) : await openExternal(l.url);
    if (!ok) refused = `This app does not open ${l.url}`;
  }
  function add(e: Event) {
    e.preventDefault();
    const u = url.trim();
    if (!u) return;
    act(api.links.add(issue.id, u, title.trim() || null, kind as LinkKind));
    url = title = '';
  }
  // The command menu focuses the same field the section's form has.
  $effect(() => {
    const go = () => { collapsed = false; queueMicrotask(() => sec?.querySelector<HTMLElement>('.add input')?.focus()); };
    addEventListener('command:issue.link.add', go);
    return () => removeEventListener('command:issue.link.add', go);
  });
</script>

<section aria-label="Links" bind:this={sec}>
  <SectionHeading name="Links" count={rows.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  <Collapse open={!collapsed}>
    {#if !rows.length}<p class="none">No links</p>{/if}
    <ul>
      {#each rows as l (l.id)}
        <li>
          <button type="button" class="go" onclick={() => open(l)}><em>{label(l.kind)}</em> <span class="t">{l.title || l.url}</span></button>
          <IconButton icon="close" label="Remove link {l.title || l.url}" onclick={() => act(api.links.remove(l.id))} />
        </li>
      {/each}
    </ul>
    {#if refused}<p role="alert" class="refused">{refused}</p>{/if}
    <form class="add" onsubmit={add}>
      <TextInput label="Link URL" placeholder="Paste a URL or a taskapp:// link" bind:value={url} />
      <TextInput label="Link title" placeholder="Title (optional)" bind:value={title} />
      <Select label="Link kind" items={KINDS} bind:value={kind} />
      <Button type="submit" variant="primary" disabled={!url.trim()}>Add</Button>
    </form>
  </Collapse>
</section>

<style>
  ul { list-style: none; }
  .none { padding: var(--s-4) var(--s-5); color: var(--muted); }
  li { display: flex; align-items: center; height: var(--row-h); padding-right: var(--s-4); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; }
  .go:hover { background: var(--active); }
  .t { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  em { color: var(--muted); font-style: normal; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; min-width: 5.5rem; }
  .refused { padding: var(--s-2) var(--s-5); color: var(--signal); }
  .add { display: grid; grid-template-columns: 2fr 1fr 8rem auto; gap: var(--s-4); padding: var(--s-4); align-items: start; }
</style>

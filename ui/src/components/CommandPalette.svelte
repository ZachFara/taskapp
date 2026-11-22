<script lang="ts">
  import { api } from '../lib/api';
  import { projects, views } from '../lib/stores.svelte';
  import { navigate } from '../app/routes';

  let { onclose, oncreate }: { onclose: () => void; oncreate: () => void } = $props();

  type Item = { id: string; label: string; hint: string; run: () => void };
  let text = $state('');
  let found = $state<Item[]>([]);
  let active = $state(0);
  let seq = 0;

  const go = (path: string) => () => navigate(path);
  const commands: Item[] = [
    { id: 'c:create', label: 'Create issue', hint: 'Command', run: () => oncreate() },
    { id: 'c:pinned', label: 'Go to Pinned', hint: 'Command', run: go('/pinned') },
    { id: 'c:cycles', label: 'Go to Cycles', hint: 'Command', run: go('/cycles') },
    { id: 'c:timeline', label: 'Go to Timeline', hint: 'Command', run: go('/projects/timeline') },
    { id: 'c:settings', label: 'Open settings', hint: 'Command', run: go('/settings/general') },
  ];

  const subsequence = (needle: string, hay: string) => {
    let i = 0;
    for (const c of hay.toLowerCase()) if (c === needle[i]) i++;
    return i === needle.length;
  };
  const local = $derived.by(() => {
    const all: Item[] = [
      ...commands,
      ...views.map((v) => ({ id: `v:${v.id}`, label: v.name, hint: 'View', run: go(`/view/${v.id}`) })),
      ...projects.map((p) => ({ id: `p:${p.id}`, label: p.name, hint: 'Project', run: go(`/project/${p.slug}`) })),
    ];
    const q = text.trim().toLowerCase();
    return q ? all.filter((x) => subsequence(q, x.label)) : all;
  });
  const items = $derived([...local, ...found]);

  async function search(q: string) {
    const mine = ++seq;
    if (!q) return void (found = []);
    const rows = await api.search.search(q, 8).catch(() => []);
    // A key such as ABC-3 is matched exactly; full-text search does not index keys.
    if (/^[a-z]+-\d+$/i.test(q)) {
      const hit = await api.issues.getByKey(q.toUpperCase()).catch(() => null);
      if (hit && !rows.some((r) => r.id === hit.id)) rows.unshift(hit);
    }
    if (mine !== seq) return;
    found = rows.map((i) => ({ id: `i:${i.id}`, label: `${i.key} ${i.title}`, hint: 'Issue', run: go(`/issue/${i.key}`) }));
  }

  function oninput() {
    active = 0;
    void search(text.trim());
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') return onclose();
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (items.length) active = (active + (e.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length;
    } else if (e.key === 'Enter') {
      const item = items[active];
      if (!item) return;
      onclose();
      item.run();
    }
  }
</script>

<div class="palette" role="dialog" aria-label="Command palette">
  <!-- svelte-ignore a11y_autofocus -->
  <input autofocus placeholder="Type a command, view, project or issue" bind:value={text} {oninput} {onkeydown} />
  <ul role="listbox">
    {#each items as item, n (item.id)}
      <li role="option" aria-selected={n === active} class:active={n === active}>
        <span>{item.label}</span><small>{item.hint}</small>
      </li>
    {:else}
      <li class="empty">No matches</li>
    {/each}
  </ul>
</div>

<style>
  .palette { position: fixed; top: 15vh; left: 50%; transform: translateX(-50%); width: 560px; background: #fff; border: 1px solid #d0d0d5; border-radius: 8px; box-shadow: 0 12px 40px #0003; z-index: 10; }
  input { width: 100%; box-sizing: border-box; padding: 12px 14px; border: 0; border-bottom: 1px solid #e4e4e8; font-size: 14px; outline: none; border-radius: 8px 8px 0 0; }
  ul { list-style: none; margin: 0; padding: 4px; max-height: 320px; overflow: auto; }
  li { display: flex; justify-content: space-between; padding: 6px 10px; border-radius: 4px; font-size: 13px; }
  li.active { background: #ececf1; }
  small, .empty { color: #777; }
</style>

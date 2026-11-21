<script lang="ts">
  import { api } from '../lib/api';
  import type { ParsedQuickAdd } from '../lib/bindings';
  import { refreshAll } from '../lib/stores.svelte';

  let { onclose }: { onclose: () => void } = $props();

  let text = $state('');
  let parsed = $state<ParsedQuickAdd | null>(null);
  let error = $state('');
  let seq = 0;

  const chips = $derived(
    parsed
      ? [
          parsed.due && `Due ${parsed.due.Date ?? parsed.due.DateTime?.utc}`,
          parsed.start_date && `Start ${parsed.start_date}`,
          parsed.priority !== null && `Priority ${parsed.priority}`,
          parsed.project_slug && `#${parsed.project_slug}`,
          ...parsed.label_names.map((l) => `@${l}`),
          parsed.estimate_minutes !== null && `${parsed.estimate_minutes}m`,
        ].filter((c): c is string => !!c)
      : [],
  );

  async function oninput() {
    error = '';
    const mine = ++seq;
    const p = text.trim() ? await api.issues.parseQuickadd(text).catch(() => null) : null;
    if (mine === seq) parsed = p;
  }

  async function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') return onclose();
    if (e.key !== 'Enter' || !text.trim()) return;
    try {
      await api.issues.createFromQuickadd(text);
    } catch (err) {
      // Keep the text so a failed create can be fixed and retried.
      error = (err as { message?: string })?.message ?? String(err);
      return;
    }
    void refreshAll().catch(() => {});
    onclose();
  }
</script>

<div class="quickadd" role="dialog" aria-label="Quick add">
  <!-- svelte-ignore a11y_autofocus -->
  <input autofocus placeholder="New issue: title !2 #project" bind:value={text} {oninput} {onkeydown} />
  {#if chips.length}
    <div class="chips">{#each chips as c (c)}<span class="chip">{c}</span>{/each}</div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
  .quickadd { position: fixed; top: 20vh; left: 50%; transform: translateX(-50%); width: 560px; background: #fff; border: 1px solid #d0d0d5; border-radius: 8px; box-shadow: 0 12px 40px #0003; z-index: 10; }
  input { width: 100%; box-sizing: border-box; padding: 12px 14px; border: 0; font-size: 14px; outline: none; border-radius: 8px; }
  .chips { display: flex; gap: 6px; padding: 0 14px 10px; }
  .chip { background: #ececf1; border-radius: 10px; padding: 2px 8px; font-size: 12px; }
  .error { margin: 0; padding: 0 14px 10px; color: #c62828; font-size: 12px; }
</style>

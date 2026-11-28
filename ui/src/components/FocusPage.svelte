<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { FocusSession, Issue, IssuePatch } from '../lib/bindings';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import Description from './detail/Description.svelte';

  // Focus mode: one issue full screen with its timer. The sidebar and toasts are off on this route (App, Shell).
  let { params }: { params: { key: string } } = $props();
  let issue = $state<Issue | null>(null);
  let session = $state<FocusSession | null>(null);
  let missing = $state(false);
  let editing = $state(false);
  let now = $state(Date.now());
  let ran = $state(false);

  const elapsed = $derived(session ? Math.max(0, Math.floor((now - Date.parse(session.started_at)) / 1000)) : 0);
  const clock = (s: number) => [Math.floor(s / 3600), Math.floor((s % 3600) / 60), s % 60].map((n, i) => (i ? String(n).padStart(2, '0') : String(n))).join(':');

  async function load(key: string) {
    try {
      const found = await api.issues.getByKey(key);
      const running = await api.focus.running();
      issue = found;
      session = running?.issue_id === found.id ? running : null;
      missing = false;
    } catch {
      missing = true;
    }
  }
  $effect(() => {
    void load(params.key);
  });
  // The clock ticks only while a session runs.
  $effect(() => {
    if (!session) return;
    const tick = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(tick);
  });

  async function start() {
    if (!issue) return;
    session = await api.focus.start(issue.id, null);
    ran = true;
    now = Date.now();
  }
  async function pause() {
    await api.focus.stop();
    session = null;
  }
  async function stop() {
    if (session) await api.focus.stop();
    navigate(`/issue/${params.key}`);
  }
  const save = async (patch: IssuePatch) => {
    if (!issue) return;
    issue = await api.issues.update(issue.id, patch);
  };
</script>

{#if issue}
  <section class="focus" aria-label="Focus on {issue.key}">
    <p class="key">{issue.key}</p>
    <h1>{issue.title}</h1>
    <div class="timer">
      <p class="clock" role="timer" aria-label="Elapsed time">{clock(elapsed)}</p>
      <div class="controls">
        {#if session}
          <Button variant="primary" onclick={pause}>Pause</Button>
        {:else}
          <Button variant="primary" onclick={start}>{ran ? 'Resume' : 'Start'}</Button>
        {/if}
        <Button onclick={stop}>Stop focus</Button>
      </div>
    </div>
    <Description {issue} {save} bind:editing />
  </section>
{:else if missing}
  <EmptyState title="No such issue">
    {#snippet action()}<Button variant="primary" onclick={() => navigate('/')}>Go to pinned</Button>{/snippet}
  </EmptyState>
{/if}

<style>
  .focus { display: grid; gap: var(--s-5); align-content: start; max-width: 60rem; margin: 0 auto; padding: var(--s-8) var(--s-6) var(--s-10); min-height: 100%; }
  .key { margin: 0; color: var(--muted); font-weight: 700; font-variant-numeric: tabular-nums; }
  h1 { margin: 0; font-size: var(--fs-display); letter-spacing: -0.03em; line-height: var(--lh-tight); }
  .timer { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: var(--s-4); padding: var(--s-5) 0; border-block: 2px solid var(--stone); }
  .clock { margin: 0; font-size: var(--fs-display); font-weight: 800; font-variant-numeric: tabular-nums; }
  .controls { display: flex; gap: var(--s-3); }
</style>

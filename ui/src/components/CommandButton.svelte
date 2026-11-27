<script lang="ts">
  import { getContext } from 'svelte';
  import { api } from '../lib/api';
  import { listCommands, type Ctx } from '../lib/commands';
  import Button from '../ui/Button.svelte';
  import IconButton from '../ui/IconButton.svelte';

  // Runs one registry command. The live context comes from App via getContext('cmdctx') as a getter.
  let {
    command,
    variant = 'quiet',
    iconOnly = false,
  }: { command: string; variant?: 'primary' | 'quiet' | 'danger'; iconOnly?: boolean } = $props();

  const fallback: Ctx = { route: '', selection: [], focusedIssueId: null, openIssueId: null, api, navigate: () => {} };
  const live = getContext<(() => Ctx) | undefined>('cmdctx');
  const ctx = () => (live ? live() : fallback);

  const cmd = $derived(listCommands(ctx(), { all: true }).find((c) => c.id === command));
  // A command whose `when` is false is hidden, the same rule the registry applies to menus and keys.
  const visible = $derived(!!cmd && (!cmd.when || cmd.when(ctx())));

  function run() {
    if (cmd) void cmd.run?.(ctx());
  }
</script>

{#if cmd && visible}
  {#if iconOnly}
    <IconButton icon={cmd.icon} label={cmd.label} shortcut={cmd.shortcut} onclick={run} />
  {:else}
    <!-- The name may hide in a page header short of room (PageHeader); aria-label keeps it for screen readers. -->
    <Button {variant} icon={cmd.icon} tooltip={{ label: cmd.label, shortcut: cmd.shortcut }} onclick={run} aria-label={cmd.label}><span class="cmd-label">{cmd.label}</span></Button>
  {/if}
{/if}

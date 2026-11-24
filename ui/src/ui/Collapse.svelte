<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { slide } from 'svelte/transition';
  import { ms } from '../lib/motion.svelte';

  // Content that eases its height open and shut (a section, a sidebar group). Off makes it instant.
  let { open, children }: { open: boolean; children: Snippet } = $props();
</script>

{#if open}
  <div transition:slide={{ duration: ms('--dur-3'), easing: cubicOut }}>{@render children()}</div>
{/if}

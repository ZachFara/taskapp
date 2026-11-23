<script lang="ts">
  import Icon from './Icon.svelte';

  // Renders a stored `icon:<id>` or `emoji:<x>` in a palette colour. A key outside the palette shows as gray.
  let { icon, color = 'gray', size = 16 }: { icon: string | null | undefined; color?: string | null; size?: number } = $props();
  const KEYS = ['gray', 'red', 'orange', 'amber', 'yellow', 'green', 'teal', 'cyan', 'blue', 'indigo', 'purple', 'pink'];
  const key = $derived(KEYS.includes(color ?? '') ? color : 'gray');
  const emoji = $derived(icon?.startsWith('emoji:') ? icon.slice(6) : null);
  const id = $derived(icon?.startsWith('icon:') ? icon.slice(5) : 'circle');
</script>

<span class="entity" style:color={`var(--c-${key})`} style:width="{size}px" style:height="{size}px" data-color={key}>
  {#if emoji}<span class="emoji" style:font-size="{size - 2}px">{emoji}</span>{:else}<Icon name={id} {size} />{/if}
</span>

<style>
  .entity { display: inline-grid; place-items: center; flex: none; }
  .emoji { line-height: 1; }
</style>

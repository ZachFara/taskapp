<script lang="ts">
  import Dialog from '../ui/Dialog.svelte';

  // `images` (detail/markdown.ts) fires `image:view` when a pasted image is clicked.
  let open = $state(false);
  let shown = $state({ src: '', alt: '' });
  $effect(() => {
    const on = (e: Event) => { shown = (e as CustomEvent).detail; open = true; };
    addEventListener('image:view', on);
    return () => removeEventListener('image:view', on);
  });
</script>

<Dialog bind:open label="Image" size="lg">
  <img class="big" src={shown.src} alt={shown.alt} />
</Dialog>

<style>
  .big { display: block; max-width: 100%; max-height: 70vh; margin: 0 auto; object-fit: contain; }
</style>

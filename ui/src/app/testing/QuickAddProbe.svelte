<script lang="ts">
  let { onclose, oncreated }: { onclose: () => void; oncreated: (i: { key: string }) => void } = $props();
</script>

<button data-quickadd onclick={() => (oncreated({ key: 'ABC-7' }), onclose())}>create</button>

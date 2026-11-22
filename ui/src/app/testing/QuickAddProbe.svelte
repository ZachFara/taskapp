<script lang="ts">
  let { onclose, oncreated }: { onclose: () => void; oncreated: (c: { issue: { key: string }; undo: () => Promise<void> }) => void } = $props();
</script>

<button data-quickadd onclick={() => (oncreated({ issue: { key: 'ABC-7' }, undo: async () => {} }), onclose())}>create</button>

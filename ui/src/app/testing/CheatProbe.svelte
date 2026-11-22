<script lang="ts">
  let { onclose }: { onclose: () => void } = $props();
</script>

<button data-cheat onclick={onclose}>close</button>

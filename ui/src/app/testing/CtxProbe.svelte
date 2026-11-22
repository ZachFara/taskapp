<script lang="ts">
  import { getContext } from 'svelte';
  import type { Ctx } from '../../lib/commands';
  const ctx = getContext<() => Ctx>('cmdctx');
</script>

<p data-probe>{ctx().route}|{ctx().openIssueId}|{ctx().selection.length}</p>

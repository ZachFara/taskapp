<script lang="ts">
  import type { Issue, IssuePatch } from '../../lib/bindings';
  import { renderMarkdown } from './markdown';

  let { issue, save }: { issue: Issue; save: (p: IssuePatch) => void } = $props();
  let editing = $state(false);
</script>

<section aria-label="Description">
  {#if editing}
    <textarea aria-label="Description text" rows="6" value={issue.description}
      onblur={(e) => {
        editing = false;
        if (e.currentTarget.value !== issue.description) save({ description: e.currentTarget.value });
      }}></textarea>
  {:else}
    <div class="md" role="button" tabindex="0" onclick={() => (editing = true)}
      onkeydown={(e) => e.key === 'Enter' && (editing = true)}>
      {#if issue.description}{@html renderMarkdown(issue.description)}{:else}<em>Add description</em>{/if}
    </div>
  {/if}
</section>

<style>
  textarea { width: 100%; }
  .md { min-height: 2em; cursor: text; }
</style>

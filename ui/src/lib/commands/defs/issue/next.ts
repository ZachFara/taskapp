import { defineCommand } from '../../registry';
import { say } from '../../../../components/undoState.svelte';
import { refreshView, targets } from './_issue';

// An issue can be the next action of its own project only; one without a project is told so.
defineCommand({
  id: 'issue.next', label: 'Make next action', icon: 'flag', group: 'Issue',
  noShortcutReason: 'Occasional action; reached from the right-click menu and the command menu.',
  run: async (ctx) => {
    const [id] = targets(ctx);
    if (id === undefined) return;
    const issue = await ctx.api.issues.get(id);
    if (issue.project_id === null) return say({ label: `${issue.key} has no project, so it cannot be a next action`, tone: 'info' });
    await ctx.api.projects.setNext(issue.project_id, id);
    refreshView();
    dispatchEvent(new CustomEvent('rows:changed'));
  },
});

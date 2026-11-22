import { defineCommand } from '../../registry';
import { targets } from './_issue';

defineCommand({
  id: 'issue.duplicate', label: 'Duplicate', icon: 'copy', group: 'Issue', shortcut: 'mod+d',
  run: async (ctx) => {
    for (const id of targets(ctx)) {
      const i = await ctx.api.issues.get(id);
      await ctx.api.issues.create({
        title: i.title, description: i.description, status_id: i.status_id, priority: i.priority, project_id: i.project_id,
        parent_id: i.parent_id, cycle_id: i.cycle_id, label_ids: i.label_ids, estimate_minutes: i.estimate_minutes,
        properties: i.properties,
      });
    }
  },
});

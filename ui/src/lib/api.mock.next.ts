// Port of crates/core/src/next_actions.rs: each unarchived project with its open next action; stuck when active and without one.
import type { Issue, Label, NextAction, Project, ProjectStatus, Status } from './bindings';

/** `flags` maps a project id to its flagged issue id. */
export function nextActions(projects: Project[], issues: Issue[], statuses: Status[], projectStatuses: ProjectStatus[], labels: Label[], flags: Map<number, number>): NextAction[] {
  return projects
    .filter((p) => !p.archived_at)
    .map((project) => {
      const issue = issues.find((i) => i.id === flags.get(project.id));
      const open = issue && issue.project_id === project.id && !issue.archived_at && !['Completed', 'Canceled'].includes(statuses.find((s) => s.id === issue.status_id)?.category ?? '');
      const started = projectStatuses.find((s) => s.id === project.status_id)?.category === 'Started';
      const place = project.label_ids.some((l) => ['area', 'collection'].includes(labels.find((x) => x.id === l)?.name.toLowerCase() ?? ''));
      return { project, issue: open ? issue : null, stuck: started && !place && !open };
    });
}

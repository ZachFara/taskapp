// Port of crates/core/src/stale.rs: issues past their status's nudge age, measured from the last status change.
import type { Change, Issue, StaleIssue, Status } from './bindings';

const DAY = 86_400_000;

export function stale(issues: Issue[], statuses: Status[], changes: Change[], now: Date): StaleIssue[] {
  const out: StaleIssue[] = [];
  for (const issue of issues) {
    const st = statuses.find((x) => x.id === issue.status_id);
    if (!st || st.nudge_days === null || issue.archived_at) continue;
    const moves = changes.filter((c) => c.entity === 'issue' && c.field === 'status_id' && c.entity_id === String(issue.id));
    const last = moves.reduce<number | null>((m, c) => (m === null || Date.parse(c.at) > m ? Date.parse(c.at) : m), null);
    const since = last ?? Date.parse(issue.updated_at);
    const waited = now.getTime() - since;
    if (waited > st.nudge_days * DAY) {
      out.push({ issue, status_name: st.name, nudge_days: st.nudge_days, since: new Date(since).toISOString(), days: Math.floor(waited / DAY) });
    }
  }
  return out.sort((a, b) => Date.parse(a.since) - Date.parse(b.since));
}

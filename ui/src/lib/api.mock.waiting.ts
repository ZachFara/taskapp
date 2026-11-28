// Port of crates/core/src/waiting.rs: open issues with a waiting note or a follow-up day; due follow-ups first.
import type { Issue, Status } from './bindings';

export function waitingList(issues: Issue[], statuses: Status[]): Issue[] {
  const open = (i: Issue) => {
    const s = statuses.find((x) => x.id === i.status_id);
    return !i.archived_at && s?.category !== 'Completed' && s?.category !== 'Canceled';
  };
  return issues
    .filter((i) => (i.waiting_on !== null || i.follow_up !== null) && open(i))
    .sort((a, b) => {
      if ((a.follow_up === null) !== (b.follow_up === null)) return a.follow_up === null ? 1 : -1;
      if (a.follow_up !== b.follow_up) return (a.follow_up ?? '') < (b.follow_up ?? '') ? -1 : 1;
      return a.id - b.id;
    })
    .map((i) => ({ ...i }));
}

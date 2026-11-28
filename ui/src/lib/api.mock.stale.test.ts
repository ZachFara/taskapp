import { describe, expect, it } from 'vitest';
import type { Change, Issue, Status } from './bindings';
import { stale } from './api.mock.stale';

const NOW = new Date('2026-11-27T12:00:00Z');
const st = (id: number, name: string, nudge_days: number | null): Status => ({ id, name, nudge_days, category: 'backlog', position: id, color: 'gray', color_hex: '#888', icon: null, definition: '', flow_kind: 'idle' }) as unknown as Status;
const issue = (id: number, status_id: number, updated_at: string, archived_at: string | null = null): Issue => ({ id, key: `ABC-${id}`, title: `t${id}`, status_id, updated_at, archived_at }) as unknown as Issue;
const move = (entity_id: number, at: string): Change => ({ id: entity_id, at, entity: 'issue', entity_id: String(entity_id), field: 'status_id', old_value: null, new_value: null }) as unknown as Change;

describe('stale issues (mock port of the core rule)', () => {
  const statuses = [st(1, 'Inbox', 7), st(2, 'Someday', null)];

  it('lists an issue past its status nudge, oldest first, with whole days', () => {
    const out = stale([issue(1, 1, '2026-11-01T00:00:00Z'), issue(2, 1, '2026-10-01T00:00:00Z')], statuses, [], NOW);
    expect(out.map((s) => s.issue.id)).toEqual([2, 1]);
    expect(out[0]).toMatchObject({ status_name: 'Inbox', nudge_days: 7, days: 57 });
  });

  it('restarts the clock on the last status change', () => {
    const out = stale([issue(1, 1, '2026-01-01T00:00:00Z')], statuses, [move(1, '2026-11-26T00:00:00Z')], NOW);
    expect(out).toEqual([]);
  });

  it('never lists a status without a nudge, or an archived issue', () => {
    const out = stale([issue(1, 2, '2020-01-01T00:00:00Z'), issue(2, 1, '2020-01-01T00:00:00Z', '2026-01-01T00:00:00Z')], statuses, [], NOW);
    expect(out).toEqual([]);
  });
});

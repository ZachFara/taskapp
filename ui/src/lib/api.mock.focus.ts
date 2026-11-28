// Port of crates/core/src/focus.rs: deep hours per bucket, their total and the streak, from the sessions.
import type { Bucket, DeepHours, FocusSession } from './bindings';
import { addDays, localDay } from './api.mock.insights';

const hours = (s: FocusSession, now: number) => Math.max(0, ((s.ended_at ? Date.parse(s.ended_at) : now) - Date.parse(s.started_at)) / 3_600_000);

/** The first day of the bucket that holds `day`: the Monday for a week, the 1st for a month. */
const bucketStart = (day: string, bucket: Bucket) => {
  if (bucket === 'Day') return day;
  if (bucket === 'Month') return `${day.slice(0, 7)}-01`;
  const back = (new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7;
  return addDays(day, -back);
};

const nextStart = (start: string, bucket: Bucket) => {
  if (bucket === 'Day') return addDays(start, 1);
  if (bucket === 'Week') return addDays(start, 7);
  const d = new Date(`${start}T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() + 1);
  return d.toISOString().slice(0, 10);
};

export function deepHours(sessions: FocusSession[], from: string, to: string, bucket: Bucket, tz: string, today: string, now = Date.now()): DeepHours {
  const worked = new Map<string, number>();
  for (const s of sessions.filter((x) => x.deep)) {
    const day = localDay(s.started_at, tz);
    worked.set(day, (worked.get(day) ?? 0) + hours(s, now));
  }
  const buckets: DeepHours['buckets'] = [];
  for (let start = bucketStart(from, bucket); start <= to; start = nextStart(start, bucket)) buckets.push({ start, hours: 0 });
  for (const [day, h] of worked) {
    const b = buckets.find((x) => x.start === bucketStart(day, bucket));
    if (day >= from && day <= to && b) b.hours = (b.hours ?? 0) + h;
  }
  let day = worked.has(today) ? today : addDays(today, -1);
  let streak = 0;
  while ((worked.get(day) ?? 0) > 0) (streak++, (day = addDays(day, -1)));
  return { buckets, total_hours: buckets.reduce((n, b) => n + (b.hours ?? 0), 0), streak_days: streak };
}

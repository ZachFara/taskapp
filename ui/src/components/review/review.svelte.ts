import { api } from '../../lib/api';
import { localDay } from '../../lib/gantt';
import { workspaceZone } from '../views/dates';

/** The Weekly review steps in order. Progress lives here, so the page and its commands share it. */
export const STEPS = ['Inbox to zero', 'Waiting', 'Stuck projects', 'Stale issues', 'Someday', 'Next 7 days'] as const;

export const review = $state({ at: 0, done: [] as number[] });

export const start = () => {
  review.at = 0;
  review.done = [];
};
export const next = () => (review.at = Math.min(review.at + 1, STEPS.length - 1));
export const back = () => (review.at = Math.max(review.at - 1, 0));
export const allDone = () => STEPS.every((_, i) => review.done.includes(i));
export const toggleDone = () => {
  const i = review.at;
  review.done = review.done.includes(i) ? review.done.filter((x) => x !== i) : [...review.done, i];
};

/** Records the review date (a day in the workspace zone) once every step is ticked, then starts over. */
export async function finish() {
  if (!allDone()) return;
  const tz = await workspaceZone();
  await api.settings.set('last_review', localDay(new Date(), tz));
  start();
}

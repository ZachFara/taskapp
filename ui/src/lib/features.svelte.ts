import { api } from './api';

/** Optional features, each a workspace switch (default on). Same names and keys as `FEATURES` in crates/core/src/settings.rs. */
export const FEATURES = [
  { name: 'dashboard', key: 'dashboard', label: 'Dashboard', blurb: 'The Dashboard page with its charts and cards.' },
  { name: 'decisions', key: 'feature_decisions', label: 'Decisions', blurb: 'Questions from agents for you to answer, with the Decisions page.' },
  { name: 'cycles', key: 'feature_cycles', label: 'Cycles', blurb: 'Time-boxed cycles: the sidebar list, the Cycles page and its commands.' },
  { name: 'events', key: 'feature_events', label: 'Events', blurb: 'Calendar events and the Events page.' },
  { name: 'charts', key: 'insights_charts', label: 'Progress charts', blurb: 'Burn-up charts on projects and cycles. The insights panel on a view still opens on demand.' },
  { name: 'stale', key: 'feature_stale', label: 'Stale nudges', blurb: 'A marker on issues that sat in a status too long, and the Stale dashboard card.' },
  { name: 'reminders', key: 'feature_reminders', label: 'Reminders capture', blurb: 'Import from an Apple Reminders list, and its Settings page.' },
  { name: 'gcal', key: 'feature_gcal', label: 'Google Calendar', blurb: 'Calendar alerts for due dates, and its Settings page.' },
  { name: 'process_inbox', key: 'feature_process_inbox', label: 'Process Inbox', blurb: 'Go through the Inbox one issue at a time: do it, delegate, defer, plan, park or trash. The Process Inbox page and its commands.' },
  { name: 'weekly_review', key: 'feature_weekly_review', label: 'Weekly review', blurb: 'The Weekly review page: a guided checklist over the Inbox, Waiting, stuck and stale issues, Someday and the next 7 days, and the Days since weekly review card.' },
  { name: 'next_actions', key: 'feature_next_actions', label: 'Next actions', blurb: 'One flagged next action per project, the Next page, and a Stuck marker and dashboard card for active projects without one.' },
  { name: 'focus', key: 'feature_focus', label: 'Focus mode', blurb: 'One issue at a time on a timer, with a focus log per issue and the Deep hours card.' },
  { name: 'recurring', key: 'feature_recurring', label: 'Recurring issues', blurb: 'A Repeat rule on an issue: finishing it creates the next one. The Repeat row and its commands.' },
  { name: 'time_blocking', key: 'feature_time_blocking', label: 'Time blocking', blurb: 'Plan the day: drag issues onto time blocks for today and tomorrow, which become calendar events. The Today plan page and its commands.' },
  { name: 'shutdown', key: 'feature_shutdown', label: 'Shutdown ritual', blurb: 'An end-of-day checklist from the Shutdown command: Inbox left, up to three deep issues for tomorrow, loose ends, and a streak. Off until switched on.' },
  { name: 'goals', key: 'feature_goals', label: 'Goals', blurb: 'Quarterly goals above your projects: the Goals page, progress from linked projects, and the goals on a project page.' },
  { name: 'waiting', key: 'feature_waiting', label: 'Waiting for', blurb: 'Who an issue waits on and a follow-up day: the Waiting page, the Follow-ups dashboard card, and the follow up marker.' },
] as const;
export type Feature = (typeof FEATURES)[number]['name'];

/** Switches that start off until someone turns them on. */
const OFF_BY_DEFAULT: string[] = ['shutdown'];

export const features = $state<Record<Feature, boolean>>(Object.fromEntries(FEATURES.map((f) => [f.name, !OFF_BY_DEFAULT.includes(f.name)])) as Record<Feature, boolean>);

/** Other code asks this before showing a feature's entry, page, command or card. */
export const isOn = (name: Feature) => features[name];

export const loadFeatures = () =>
  Promise.all(FEATURES.map((f) => api.settings.get(f.key).then((v) => (features[f.name] = typeof v === 'boolean' ? v : !OFF_BY_DEFAULT.includes(f.name)), () => {})));

export async function setFeature(name: Feature, on: boolean) {
  features[name] = on;
  try {
    await api.settings.set(FEATURES.find((f) => f.name === name)!.key, on);
  } catch (e) {
    features[name] = !on;
    throw e;
  }
}

/** Commands and pages that belong to a feature; the registry hides a command whose feature is off. */
const COMMANDS: [RegExp, Feature][] = [
  [/^(nav\.dashboard|dashboard\.)/, 'dashboard'],
  [/^(nav\.decisions|decision\.)/, 'decisions'],
  [/^(nav\.cycles|cycle\.|settings\.cycles$|issue\.cycle$)/, 'cycles'],
  [/^(nav\.events|event\.|issue\.event\.)/, 'events'],
  [/^(chart\.|view\.insights$)/, 'charts'],
  [/^(nav\.next|issue\.next|project\.next)$/, 'next_actions'],
  [/^(nav\.process|process\.)/, 'process_inbox'],
  [/^(nav\.review|review\.)/, 'weekly_review'],
  [/^settings\.reminders$/, 'reminders'],
  [/^settings\.calendar$/, 'gcal'],
  [/^(issue\.focus$|focus\.)/, 'focus'],
  [/^issue\.repeat/, 'recurring'],
  [/^(nav\.plan|plan\.)/, 'time_blocking'],
  [/^shutdown\./, 'shutdown'],
  [/^(nav\.goals|goal\.)/, 'goals'],
  [/^(nav\.waiting|issue\.wait|issue\.followUp)/, 'waiting'],
];
export const featureOfCommand = (id: string) => COMMANDS.find(([re]) => re.test(id))?.[1];

export const ROUTES: Record<string, Feature> = { '/focus': 'focus', '/dashboard': 'dashboard', '/decisions': 'decisions', '/cycles': 'cycles', '/events': 'events', '/event': 'events', '/next': 'next_actions', '/process': 'process_inbox', '/plan': 'time_blocking', '/review': 'weekly_review', '/goals': 'goals', '/goal': 'goals', '/waiting': 'waiting' };
export const SETTINGS_PAGES: Record<string, Feature> = { cycles: 'cycles', reminders: 'reminders', calendar: 'gcal' };

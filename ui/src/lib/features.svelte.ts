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
  { name: 'process_inbox', key: 'feature_process_inbox', label: 'Process Inbox', blurb: 'Coming: a guided pass that clears the inbox.' },
  { name: 'next_actions', key: 'feature_next_actions', label: 'Next actions', blurb: 'Coming: a list of the next step of each project.' },
  { name: 'focus', key: 'feature_focus', label: 'Focus mode', blurb: 'Coming: one issue at a time, nothing else on screen.' },
] as const;
export type Feature = (typeof FEATURES)[number]['name'];

export const features = $state<Record<Feature, boolean>>(Object.fromEntries(FEATURES.map((f) => [f.name, true])) as Record<Feature, boolean>);

/** Other code asks this before showing a feature's entry, page, command or card. */
export const isOn = (name: Feature) => features[name];

export const loadFeatures = () =>
  Promise.all(FEATURES.map((f) => api.settings.get(f.key).then((v) => (features[f.name] = v !== false), () => {})));

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
  [/^settings\.reminders$/, 'reminders'],
  [/^settings\.calendar$/, 'gcal'],
];
export const featureOfCommand = (id: string) => COMMANDS.find(([re]) => re.test(id))?.[1];

export const ROUTES: Record<string, Feature> = { '/dashboard': 'dashboard', '/decisions': 'decisions', '/cycles': 'cycles', '/events': 'events', '/event': 'events' };
export const SETTINGS_PAGES: Record<string, Feature> = { cycles: 'cycles', reminders: 'reminders', calendar: 'gcal' };

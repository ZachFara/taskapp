import { isOn, type Feature } from '../../lib/features.svelte';
import { CARDS } from './cards';

const KEY = 'cairn.dashboard';
type Saved = { range: string; from: string; to: string; measure: 'Count' | 'Points'; order: string[]; hidden: string[] };
const FRESH: Saved = { range: 'week', from: '', to: '', measure: 'Count', order: [], hidden: [] };

function read(): Saved {
  try {
    return { ...FRESH, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') };
  } catch {
    return FRESH;
  }
}

/** Per-device choices (range, measure, cards). The workspace switch is the `dashboard` feature. Filters live on the page. */
export const dashboard = $state({ ...read(), project: null as number | null, label: null as number | null });

function save() {
  const { range, from, to, measure, order, hidden } = dashboard;
  try {
    localStorage.setItem(KEY, JSON.stringify({ range, from, to, measure, order, hidden }));
  } catch {
    /* private window: the choice applies, it is just not remembered */
  }
}
export const set = (patch: Partial<Saved> & { project?: number | null; label?: number | null }) => (Object.assign(dashboard, patch), save());

/** Cards that belong to another feature disappear with it. */
const CARD_FEATURE: Record<string, Feature> = { stale: 'stale', cycle: 'cycles', stuck: 'next_actions', deep: 'focus', review: 'weekly_review' };

/** Saved order first, then any card the saved order has not heard of (a later ticket's) in registry order. */
export const orderedCards = () => {
  const known = CARDS.filter((c) => !(CARD_FEATURE[c.id] && !isOn(CARD_FEATURE[c.id]))).map((c) => c.id);
  return [...dashboard.order.filter((id) => known.includes(id)), ...known.filter((id) => !dashboard.order.includes(id))];
};
export const toggleCard = (id: string) => set({ hidden: dashboard.hidden.includes(id) ? dashboard.hidden.filter((x) => x !== id) : [...dashboard.hidden, id] });
export function moveCard(id: string, by: -1 | 1) {
  const o = orderedCards();
  const i = o.indexOf(id);
  const j = i + by;
  if (i < 0 || j < 0 || j >= o.length) return;
  [o[i], o[j]] = [o[j], o[i]];
  set({ order: o });
}

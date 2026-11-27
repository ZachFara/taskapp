import type { Component } from 'svelte';
import type { Metrics } from './metrics';
import BurnupCard from './BurnupCard.svelte';
import SummaryCard from './SummaryCard.svelte';

export type CardProps = { metrics: Metrics; measure: 'Count' | 'Points' };
/** One entry per card; the later tickets (MIT-328 to 334) add theirs here. A card never fetches: it draws `metrics`. */
export const CARDS: { id: string; title: string; wide?: boolean; component: Component<CardProps> }[] = [
  { id: 'summary', title: 'Summary', component: SummaryCard },
  { id: 'burnup', title: 'Burn-up', wide: true, component: BurnupCard },
];

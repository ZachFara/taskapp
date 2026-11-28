import type { Component } from 'svelte';
import type { Metrics } from './metrics';
import BurnupCard from './BurnupCard.svelte';
import SummaryCard from './SummaryCard.svelte';
import CycleTimeCard from './CycleTimeCard.svelte';
import LeadTimeCard from './LeadTimeCard.svelte';
import WipCard from './WipCard.svelte';

export type CardProps = { metrics: Metrics; measure: 'Count' | 'Points' };
/** One entry per card; the later tickets (MIT-328 to 334) add theirs here. A card never fetches: it draws `metrics`. */
export const CARDS: { id: string; title: string; wide?: boolean; component: Component<CardProps> }[] = [
  { id: 'summary', title: 'Summary', component: SummaryCard },
  { id: 'burnup', title: 'Burn-up', wide: true, component: BurnupCard },
  { id: 'cycletime', title: 'Cycle time', component: CycleTimeCard },
  { id: 'leadtime', title: 'Lead time', component: LeadTimeCard },
  { id: 'wip', title: 'Work in progress', wide: true, component: WipCard },
];

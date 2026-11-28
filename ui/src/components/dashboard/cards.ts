import type { Component } from 'svelte';
import type { Filter } from '../../lib/bindings';
import type { Metrics } from './metrics';
import AiVsMeCard from './AiVsMeCard.svelte';
import BurnupCard from './BurnupCard.svelte';
import ByLabelCard from './ByLabelCard.svelte';
import CreatedDoneCard from './CreatedDoneCard.svelte';
import ForecastCard from './ForecastCard.svelte';
import SummaryCard from './SummaryCard.svelte';
import CycleTimeCard from './CycleTimeCard.svelte';
import LeadTimeCard from './LeadTimeCard.svelte';
import WipCard from './WipCard.svelte';
import ThroughputCard from './ThroughputCard.svelte';
import VelocityCard from './VelocityCard.svelte';

export type CardProps = { metrics: Metrics; measure: 'Count' | 'Points'; filter: Filter | null; cycle: number | null };
/** One entry per card; the later tickets (MIT-328 to 334) add theirs here. A card never fetches: it draws `metrics`. */
export const CARDS: { id: string; title: string; wide?: boolean; component: Component<CardProps> }[] = [
  { id: 'summary', title: 'Summary', component: SummaryCard },
  { id: 'burnup', title: 'Burn-up', wide: true, component: BurnupCard },
  { id: 'cycletime', title: 'Cycle time', component: CycleTimeCard },
  { id: 'leadtime', title: 'Lead time', component: LeadTimeCard },
  { id: 'wip', title: 'Work in progress', wide: true, component: WipCard },
  { id: 'throughput', title: 'Throughput', component: ThroughputCard },
  { id: 'created-done', title: 'Created vs completed', component: CreatedDoneCard },
  { id: 'velocity', title: 'Velocity', component: VelocityCard },
  { id: 'ai-vs-me', title: 'AI vs me', component: AiVsMeCard },
  { id: 'by-label', title: 'By label', component: ByLabelCard },
  { id: 'forecast', title: 'Forecast', component: ForecastCard },
];

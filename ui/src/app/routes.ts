import type { Component } from 'svelte';
import ArchivePage from '../components/ArchivePage.svelte';
import DecisionsPage from '../components/DecisionsPage.svelte';
import DashboardPage from '../components/dashboard/DashboardPage.svelte';
import CyclesPage from '../components/CyclesPage.svelte';
import InboxPage from '../components/InboxPage.svelte';
import FocusPage from '../components/FocusPage.svelte';
import IssueDetail from '../components/IssueDetail.svelte';
import NextPage from '../components/NextPage.svelte';
import MyIssuesPage from '../components/MyIssuesPage.svelte';
import PlanPage from '../components/PlanPage.svelte';
import ProcessPage from '../components/ProcessPage.svelte';
import ReviewPage from '../components/ReviewPage.svelte';
import PinnedPage from '../components/PinnedPage.svelte';
import ProjectPage from '../components/ProjectPage.svelte';
import ProjectsPage from '../components/ProjectsPage.svelte';
import ViewsPage from '../components/ViewsPage.svelte';
import EventPage from '../components/events/EventPage.svelte';
import EventsPage from '../components/events/EventsPage.svelte';
import Settings from '../components/settings/Settings.svelte';
import GanttView from '../components/views/GanttView.svelte';
import ViewPage from './ViewPage.svelte';
import DesignGallery from './DesignGallery.svelte';

export const routes: { pattern: string; component: Component }[] = [
  { pattern: '/', component: PinnedPage }, // the app window opens on '/'
  { pattern: '/view/:id', component: ViewPage },
  { pattern: '/view/:id/:layout', component: ViewPage },
  { pattern: '/issue/:key', component: IssueDetail },
  { pattern: '/focus/:key', component: FocusPage },
  { pattern: '/project/:slug', component: ProjectPage },
  { pattern: '/projects/timeline', component: GanttView },
  { pattern: '/pinned', component: PinnedPage },
  { pattern: '/archive', component: ArchivePage },
  { pattern: '/cycles', component: CyclesPage },
  { pattern: '/dashboard', component: DashboardPage },
  { pattern: '/decisions', component: DecisionsPage },
  { pattern: '/next', component: NextPage },
  { pattern: '/events', component: EventsPage },
  { pattern: '/event/:id', component: EventPage },
  { pattern: '/settings/:section', component: Settings },
  { pattern: '/design', component: DesignGallery },
  { pattern: '/inbox', component: InboxPage },
  { pattern: '/process', component: ProcessPage },
  { pattern: '/review', component: ReviewPage },
  { pattern: '/plan', component: PlanPage },
  { pattern: '/my', component: MyIssuesPage },
  { pattern: '/projects', component: ProjectsPage },
  { pattern: '/views', component: ViewsPage },
];

/** The first route whose pattern fits `path`, with its `:name` segments as params. */
export function matchRoute(path: string) {
  const parts = path.split('/');
  for (const { pattern, component } of routes) {
    const want = pattern.split('/');
    if (want.length !== parts.length) continue;
    const params: Record<string, string> = {};
    const ok = want.every((w, i) => {
      if (w.startsWith(':')) return (params[w.slice(1)] = decodeURIComponent(parts[i])) !== '';
      return w === parts[i];
    });
    if (ok) return { component, params };
  }
  return null;
}

export function navigate(path: string) {
  history.pushState(null, '', path);
  dispatchEvent(new PopStateEvent('popstate'));
}

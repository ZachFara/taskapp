import type { Component } from 'svelte';
import CyclesPage from '../components/CyclesPage.svelte';
import IssueDetail from '../components/IssueDetail.svelte';
import PinnedPage from '../components/PinnedPage.svelte';
import ProjectPage from '../components/ProjectPage.svelte';
import Settings from '../components/settings/Settings.svelte';
import GanttView from '../components/views/GanttView.svelte';
import ViewPage from './ViewPage.svelte';
import DesignGallery from './DesignGallery.svelte';

export const routes: { pattern: string; component: Component }[] = [
  { pattern: '/', component: PinnedPage }, // the app window opens on '/'
  { pattern: '/view/:id', component: ViewPage },
  { pattern: '/view/:id/:layout', component: ViewPage },
  { pattern: '/issue/:key', component: IssueDetail },
  { pattern: '/project/:slug', component: ProjectPage },
  { pattern: '/projects/timeline', component: GanttView },
  { pattern: '/pinned', component: PinnedPage },
  { pattern: '/cycles', component: CyclesPage },
  { pattern: '/settings/:section', component: Settings },
  { pattern: '/design', component: DesignGallery },
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

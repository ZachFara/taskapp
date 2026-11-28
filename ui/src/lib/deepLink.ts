import type { Api } from './api';

/** Every item type that has a `taskapp://<kind>/<ref>` link. */
export type LinkKind = 'issue' | 'project' | 'view' | 'cycle' | 'milestone' | 'event' | 'decision';

export const linkTo = (kind: LinkKind, ref: string | number) => `taskapp://${kind}/${ref}`;

export const copyLink = (kind: LinkKind, ref: string | number) => navigator.clipboard.writeText(linkTo(kind, ref));

export const isCairnLink = (url: string) => url.startsWith('taskapp://');

/**
 * The app route a `taskapp://` link opens, or null when it names nothing here. A milestone lives on its project's page,
 * so it is found by asking each project; a cycle opens its retrospective through the hash the Cycles page reads.
 */
export async function routeOf(url: string, api: Pick<Api, 'projects'>): Promise<string | null> {
  const m = /^taskapp:\/\/([a-z]+)\/([^/?#]+)\/?$/.exec(url);
  if (!m) return null;
  const [, kind, ref] = m;
  const enc = encodeURIComponent(ref);
  switch (kind) {
    case 'issue': return `/issue/${enc}`;
    case 'project': return `/project/${enc}`;
    case 'view': return `/view/${enc}`;
    case 'event': return `/event/${enc}`;
    case 'cycle': return `/cycles#${enc}`;
    case 'decision': return `/decisions#${enc}`;
    case 'milestone':
      for (const p of await api.projects.list()) if ((await api.projects.listMilestones(p.id)).some((x) => String(x.id) === ref)) return `/project/${p.slug}`;
      return null;
    default: return null;
  }
}

/** Goes to what a `taskapp://` link names; false when it names nothing. */
export async function openCairnLink(url: string, api: Pick<Api, 'projects'>): Promise<boolean> {
  const route = await routeOf(url, api);
  if (!route) return false;
  history.pushState(null, '', route);
  dispatchEvent(new PopStateEvent('popstate'));
  return true;
}

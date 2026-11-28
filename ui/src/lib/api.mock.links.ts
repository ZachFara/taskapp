// Port of crates/core/src/links.rs, so the mock keeps issue links exactly as the backend does.
import type { Api } from './api';
import type { Link } from './bindings';

type Fail = (kind: string, message: string) => never;

const SCHEMES = ['http', 'https', 'mailto', 'obsidian', 'taskapp'];

export function linksApi(rows: Link[], issueExists: (id: number) => void, fail: Fail): Api['links'] {
  return {
    list: async (issue) => rows.filter((l) => l.issue_id === issue).map((l) => ({ ...l })),
    add: async (issue, rawUrl, title, kind) => {
      const url = rawUrl.trim();
      const scheme = url.includes(':') ? url.slice(0, url.indexOf(':')).toLowerCase() : '';
      if (!SCHEMES.includes(scheme) || /\s/.test(url)) fail('Invalid', `a link must be one URL starting with ${SCHEMES.join(', ')}`);
      issueExists(issue);
      let link = rows.find((l) => l.issue_id === issue && l.url === url);
      if (!link) {
        link = { id: Math.max(0, ...rows.map((r) => r.id)) + 1, issue_id: issue, url, title: (title ?? '').trim(), kind: kind ?? 'other', created_at: new Date().toISOString() };
        rows.push(link);
      }
      return { ...link };
    },
    remove: async (id) => {
      const i = rows.findIndex((l) => l.id === id);
      if (i >= 0) rows.splice(i, 1);
      return null;
    },
  };
}

// Port of crates/core/src/decisions.rs, so the mock keeps decisions exactly as the backend does.
import type { Api } from './api';
import type { Decision } from './bindings';

type Fail = (kind: string, message: string) => never;

export function decisionsApi(rows: Decision[], issueExists: (id: number) => void, fail: Fail): Api['decisions'] {
  const get = (id: number) => rows.find((d) => d.id === id) ?? fail('NotFound', `decision ${id}`);
  const open = (id: number) => {
    const d = get(id);
    if (d.status !== 'open') fail('Invalid', `decision ${id} is not open`);
    return d;
  };
  const link = (d: Decision, issue: number) => {
    issueExists(issue);
    if (!d.issue_ids.includes(issue)) d.issue_ids = [...d.issue_ids, issue].sort((a, b) => a - b);
  };
  return {
    list: async (status) =>
      rows
        .filter((d) => status === null || d.status === status)
        .sort((a, b) => Number(b.mode === 'blocking') - Number(a.mode === 'blocking') || b.id - a.id)
        .map((d) => ({ ...d })),
    get: async (id) => ({ ...get(id) }),
    create: async (input) => {
      if (!input.question.trim()) fail('Invalid', 'a decision needs a question');
      const keys = new Set<string>();
      for (const o of input.options) {
        if (!o.key.trim() || !o.label.trim()) fail('Invalid', 'every option needs a key and a label');
        if (keys.has(o.key)) fail('Invalid', `option key ${o.key} is used twice`);
        keys.add(o.key);
      }
      if (input.recommendation !== null && !keys.has(input.recommendation)) fail('Invalid', `recommendation ${input.recommendation} is not one of the options`);
      const source = input.source ?? 'ui';
      if (input.external_id !== null && rows.some((d) => d.source === source && d.external_id === input.external_id))
        fail('Invalid', `source ${source} already raised a decision with that external id`);
      const d: Decision = {
        id: Math.max(0, ...rows.map((r) => r.id)) + 1, question: input.question, context: input.context, options: input.options.map((o) => ({ ...o })),
        recommendation: input.recommendation, mode: input.mode, status: 'open', answer: null, note: '', source, external_id: input.external_id,
        created_at: new Date().toISOString(), answered_at: null, issue_ids: [],
      };
      input.issue_ids.forEach((i) => link(d, i));
      rows.push(d);
      return { ...d };
    },
    answer: async (id, option, text, note) => {
      if ((option === null) === (text === null) || text?.trim() === '') fail('Invalid', 'answer with exactly one of an option or some text');
      const d = open(id);
      if (option !== null && d.options.every((o) => o.key !== option)) fail('Invalid', `${option} is not one of the options`);
      Object.assign(d, { status: 'answered', answer: option ?? text, note, answered_at: new Date().toISOString() });
      return { ...d };
    },
    withdraw: async (id) => {
      const d = open(id);
      Object.assign(d, { status: 'withdrawn', answer: null, note: '', answered_at: new Date().toISOString() });
      return { ...d };
    },
    link: async (decision, issue) => (link(get(decision), issue), null),
    unlink: async (decision, issue) => {
      const d = get(decision);
      d.issue_ids = d.issue_ids.filter((x) => x !== issue);
      return null;
    },
  };
}

/** What the dev harness (`dev:mock`) starts with; tests and the contract cases start empty. */
export const demoDecisions: Decision[] = [
  {
    id: 2, question: 'Should the sync run every minute or only when the window gains focus?', mode: 'blocking', status: 'open', source: 'agent:builder', external_id: null,
    context: 'Syncing on focus is cheaper but a change made on the phone shows up late.\n\nThe sync itself takes about two seconds.',
    options: [
      { key: 'a', label: 'Every minute', tradeoff: 'Always fresh; wakes the laptop and uses battery.' },
      { key: 'b', label: 'On focus', tradeoff: 'Cheap; a phone change waits until you open the app.' },
    ],
    recommendation: 'b', answer: null, note: '', created_at: '2026-11-27T09:00:00Z', answered_at: null, issue_ids: [1],
  },
  {
    id: 1, question: 'Name the export folder after the workspace?', mode: 'assume', status: 'open', source: 'cli', external_id: null,
    context: '', options: [{ key: 'yes', label: 'Yes', tradeoff: 'Easy to find.' }, { key: 'no', label: 'No', tradeoff: 'Keeps the old path.' }],
    recommendation: 'no', answer: null, note: '', created_at: '2026-11-26T09:00:00Z', answered_at: null, issue_ids: [],
  },
  {
    id: 3, question: 'Keep the old import log?', mode: 'assume', status: 'answered', source: 'cli', external_id: null,
    context: '', options: [{ key: 'keep', label: 'Keep', tradeoff: '' }, { key: 'drop', label: 'Drop', tradeoff: '' }],
    recommendation: 'keep', answer: 'keep', note: 'Needed for undo.', created_at: '2026-11-25T09:00:00Z', answered_at: '2026-11-25T10:00:00Z', issue_ids: [],
  },
];

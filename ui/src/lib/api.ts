import { commands } from './bindings';
// The generated file's own text gives each command's wire name and argument names for the http transport.
import bindingsSource from './bindings.ts?raw';
import { createMockApi, fixtures } from './api.mock';
import { demoDecisions } from './api.mock.decisions';

type Cmds = typeof commands;
// `issuesGetByKey` -> ["issues", "getByKey"]
type Split<K extends string, Head extends string = ''> = K extends `${infer C}${infer R}`
  ? C extends Lowercase<C>
    ? Split<R, `${Head}${C}`>
    : [Head, Uncapitalize<K>]
  : [Head, ''];
type Mod = Split<keyof Cmds & string>[0];

/** Every command grouped by module: `api.issues.getByKey(key)`. Same signatures as `commands`. */
export type Api = {
  [M in Mod]: { [K in keyof Cmds as Split<K & string>[0] extends M ? Split<K & string>[1] : never]: Cmds[K] };
};

type Call = (key: string, args: unknown[]) => Promise<unknown>;

function build(call: Call): Api {
  const api: Record<string, Record<string, unknown>> = {};
  for (const key of Object.keys(commands)) {
    const [, mod, rest] = key.match(/^([a-z]+)(.*)$/)!;
    (api[mod] ??= {})[rest[0].toLowerCase() + rest.slice(1)] = (...args: unknown[]) => call(key, args);
  }
  return api as Api;
}

const snake = (s: string) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);

// camelCase binding name -> [command, argument names in order]
const wire = new Map<string, [string, string[]]>(
  [...bindingsSource.matchAll(/^\t(\w+): .*__TAURI_INVOKE<.*?>\("(\w+)", (?:undefined|\{([^}]*)\})/gm)].map(
    ([, key, cmd, names]) => [key, [cmd, names ? names.split(',').map((n) => n.trim()) : []]],
  ),
);

export function httpApi(base: string = import.meta.env.VITE_API_URL ?? 'http://127.0.0.1:7878'): Api {
  return watchWrites(build(async (key, args) => {
    const [cmd, names] = wire.get(key)!;
    const body = Object.fromEntries(names.map((n, i) => [snake(n), args[i] ?? null]));
    const res = await fetch(`${base}/rpc/${cmd}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    });
    const json = await res.json();
    if (json.error) throw json.error;
    return json.ok;
  }));
}

// Reads (and a layout pick) never change history; undo and redo show their own toast, so a second one would overwrite it.
const READ = /^(get|list|kindsList|statusesList|query|search|similar|status(?![a-z])|progress|children|parse|plan|for|bars|criticalPath|between|undoStatus|items$|stats$|velocity$|setLayout$)/;
const watched = new WeakSet<Api>();

/** Makes every write tell the app (`history:wrote`) once it succeeds, so the undo toast can name the step. Wraps in place. */
function watchWrites(a: Api): Api {
  if (watched.has(a)) return a;
  watched.add(a);
  for (const [mod, fns] of Object.entries(a) as [string, Record<string, (...args: unknown[]) => Promise<unknown>>][]) {
    if (mod === 'history') continue;
    for (const [name, fn] of Object.entries(fns)) {
      if (READ.test(name)) continue;
      fns[name] = async (...args) => {
        const out = await fn(...args);
        window.dispatchEvent(new Event('history:wrote'));
        return out;
      };
    }
  }
  return a;
}

const mode = import.meta.env.VITE_API_MODE;

export let api: Api = watchWrites(
  mode === 'mock' ? createMockApi(location.search.includes('fresh') ? { ...fixtures, workspace: { setup_done: false, name: null, key: 'ABC', timezone: 'UTC' } } : { ...fixtures, decisions: structuredClone(demoDecisions) }) : mode === 'http' ? httpApi() : build((key, args) => (commands as any)[key](...args)),
);

/** Swaps the implementation; tests use it to install the mock. */
export function setApi(impl: Api) {
  api = watchWrites(impl);
}

// Runs a case table against the mock and against a real devserver; both sides see the same calls in the same order.
import { type ChildProcess, execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import bindingsSource from '../bindings.ts?raw';
import { createMockApi, type MockApi } from '../api.mock';
import type { Case } from './cases';

export type Outcome = { ok: unknown } | { error: { kind: string; message: string } };

const snake = (s: string) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
// wire command -> [module, function, argument names in order]
const wire = new Map<string, [string, string, string[]]>(
  [...bindingsSource.matchAll(/^\t(\w+): .*__TAURI_INVOKE<.*?>\("(\w+)", (?:undefined|\{([^}]*)\})/gm)].map(([, key, cmd, names]) => {
    const [, mod, rest] = key.match(/^([a-z]+)(.*)$/)!;
    return [cmd, [mod, rest[0].toLowerCase() + rest.slice(1), names ? names.split(',').map((n) => n.trim()) : []]];
  }),
);

// Timestamps and nothing else: the two sides run at different moments. Ids are kept because both start from the same seed.
const isTime = (k: string) => k === 'at' || k === 'since' || k.endsWith('_at');
export function normalise(v: unknown, key = ''): unknown {
  if (Array.isArray(v)) return v.map((x) => normalise(x));
  // A history row's id counts every entity's changes, which the mock does not keep.
  if (v && typeof v === 'object' && 'entity_id' in v && 'old_value' in v) return normalise({ ...v, id: '<id>' });
  // A stale row's day count follows the clock, like its since: the two sides run at different moments.
  if (v && typeof v === 'object' && 'nudge_days' in v && 'since' in v) return normalise({ ...v, days: '<days>' });
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, normalise(x, k)]));
  return isTime(key) && typeof v === 'string' ? '<time>' : v;
}

const settle = async (run: () => Promise<unknown>): Promise<Outcome> => {
  try {
    return { ok: normalise(JSON.parse(JSON.stringify((await run()) ?? null))) };
  } catch (e) {
    const x = e as { kind?: string; message?: string };
    return { error: { kind: x.kind ?? 'Unknown', message: x.message ?? String(e) } };
  }
};

export async function runMock(cases: Case[]): Promise<Outcome[]> {
  const api = createMockApi(undefined, 'UTC') as MockApi & Record<string, Record<string, (...a: unknown[]) => Promise<unknown>>>;
  const out: Outcome[] = [];
  for (const c of cases) {
    out.push(await settle(async () => {
      if (c.cmd === 'workspace_status') return api.workspace.status();
      if (c.cmd === 'workspace_setup') return api.workspace.setup(c.args!.name as string, c.args!.key as string, (c.args!.timezone as string | undefined) ?? null);
      const [mod, fn, names] = wire.get(c.cmd) ?? [c.cmd, c.cmd, []];
      return api[mod][fn](...names.map((n) => c.args?.[snake(n)] ?? null));
    }));
  }
  return out;
}

export const devserverBin = resolve(import.meta.dirname, '../../../../target/debug/taskapp-devserver');
export const haveDevserver = existsSync(devserverBin);

/** Seeds a temp db, serves it on a free port with TZ=UTC, runs the cases, stops the server. */
export async function runReal(cases: Case[]): Promise<Outcome[]> {
  const db = join(mkdtempSync(join(tmpdir(), 'contract-')), 'contract.db');
  execFileSync(devserverBin, ['seed', '--db', db]);
  const server: ChildProcess = spawn(devserverBin, ['serve', '--db', db, '--port', '0'], { env: { ...process.env, TZ: 'UTC' }, stdio: ['ignore', 'pipe', 'inherit'] });
  try {
    const base = await new Promise<string>((ok, fail) => {
      server.once('exit', () => fail(new Error('devserver exited before listening')));
      server.stdout!.on('data', (d: Buffer) => {
        const m = /listening on (\S+)/.exec(String(d));
        if (m) ok(`http://${m[1]}`);
      });
    });
    const out: Outcome[] = [];
    for (const c of cases) {
      out.push(await settle(async () => {
        const res = await fetch(`${base}/rpc/${c.cmd}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(c.args ?? {}) });
        const json = await res.json();
        if (json.error) throw json.error;
        return json.ok;
      }));
    }
    return out;
  } finally {
    server.kill();
  }
}

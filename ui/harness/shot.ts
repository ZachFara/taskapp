// Screenshot harness: `pnpm -C ui shot <route|url> [--mode mock|real] [--width] [--height] [--keys] [--click] [--out]`.
// Writes <out>/<slug>.png, .aria.txt and .console.json; exits 1 on any console error or failed request.
import { type ChildProcess, spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from '@playwright/test';

const uiDir = resolve(import.meta.dirname, '..');
const repoDir = resolve(uiDir, '..');
// Real mode needs 5173: the devserver only allows that origin. Mock gets its own port so both can run.
const modes = { mock: { vite: 5174 }, real: { vite: 5173 } } as const;
const DEV_PORT = 7878;

const up = (url: string, init?: RequestInit) =>
  fetch(url, init).then(() => true, () => false);

async function waitFor(url: string, init?: RequestInit) {
  for (let i = 0; i < 300; i++) {
    if (await up(url, init)) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`timed out waiting for ${url}`);
}

const started: ChildProcess[] = [];
function launch(cmd: string, args: string[], cwd: string, env: Record<string, string> = {}) {
  // Own process group so the whole tree (pnpm, vite, cargo) can be stopped.
  const p = spawn(cmd, args, { cwd, env: { ...process.env, ...env }, stdio: ['ignore', 'inherit', 'inherit'], detached: true });
  started.push(p);
  return p;
}
function stopAll() {
  for (const p of started) if (p.pid) try { process.kill(-p.pid); } catch { /* already gone */ }
}

const probe = `http://127.0.0.1:${DEV_PORT}/rpc/settings_get`;
const probeInit = { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{"key":"issue_prefix"}' };

async function ensureServers(mode: 'mock' | 'real') {
  const base = `http://localhost:${modes[mode].vite}`;
  if (mode === 'real' && !(await up(probe, probeInit))) {
    const db = join(mkdtempSync(join(tmpdir(), 'taskapp-shot-')), 'shot.db');
    const run = (sub: string[]) => ['run', '-q', '-p', 'taskapp-devserver', '--', ...sub, '--db', db];
    await new Promise<void>((ok, fail) =>
      spawn('cargo', run(['seed']), { cwd: repoDir, stdio: 'inherit' }).on('exit', (c) =>
        c === 0 ? ok() : fail(new Error('seed failed')),
      ),
    );
    launch('cargo', run(['serve']), repoDir);
    await waitFor(probe, probeInit);
  }
  if (!(await up(base))) {
    const env = { VITE_API_MODE: mode === 'mock' ? 'mock' : 'http' };
    launch('pnpm', ['exec', 'vite', '--host', 'localhost', '--port', String(modes[mode].vite), '--strictPort'], uiDir, env);
    await waitFor(base);
  }
  return base;
}

const slug = (route: string) => route.replace(/^https?:\/\//, '').replace(/[^a-zA-Z0-9]+/g, '-').replace(/^-|-$/g, '') || 'root';
const isKeyName = (t: string) => /^[A-Z][A-Za-z0-9+]+$/.test(t);

async function main() {
  const { values, positionals } = parseArgs({
    allowPositionals: true,
    options: {
      mode: { type: 'string', default: 'mock' },
      width: { type: 'string', default: '1440' },
      height: { type: 'string', default: '900' },
      keys: { type: 'string' },
      click: { type: 'string' },
      out: { type: 'string', default: join(uiDir, 'harness-out') },
    },
  });
  const route = positionals[0];
  if (!route || (values.mode !== 'mock' && values.mode !== 'real')) {
    throw new Error('usage: shot <route|url> [--mode mock|real] [--width N] [--height N] [--keys "a,b,Enter"] [--click selector]');
  }
  // A full URL is opened as is, with no dev server started.
  const url = /^https?:\/\//.test(route) ? route : (await ensureServers(values.mode)) + route;

  const problems: { type: string; text: string }[] = [];
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: Number(values.width), height: Number(values.height) } });
    // The browser asks for /favicon.ico on its own; the app has none.
    const noise = (u: string) => u.endsWith('/favicon.ico');
    page.on('console', (m) => m.type() === 'error' && problems.push({ type: 'console.error', text: m.text() }));
    page.on('pageerror', (e) => problems.push({ type: 'pageerror', text: e.message }));
    page.on('requestfailed', (r) => noise(r.url()) || problems.push({ type: 'requestfailed', text: `${r.url()} ${r.failure()?.errorText}` }));
    page.on('response', (r) => r.status() >= 400 && !noise(r.url()) && problems.push({ type: 'http', text: `${r.status()} ${r.url()}` }));

    await page.goto(url, { waitUntil: 'networkidle' });
    if (values.click) await page.click(values.click);
    for (const token of (values.keys ?? '').split(',').filter(Boolean)) {
      if (isKeyName(token)) await page.keyboard.press(token);
      else await page.keyboard.type(token);
    }
    await page.waitForLoadState('networkidle');

    mkdirSync(values.out, { recursive: true });
    const base = join(values.out, slug(route));
    await page.screenshot({ path: `${base}.png` });
    writeFileSync(`${base}.aria.txt`, await page.locator('body').ariaSnapshot());
    writeFileSync(`${base}.console.json`, JSON.stringify(problems, null, 2));
    console.log(`${base}.png (${problems.length} problems)`);
  } finally {
    await browser.close();
  }
  return problems.length === 0 ? 0 : 1;
}

main().then(
  (code) => (stopAll(), process.exit(code)),
  (e) => (console.error(e instanceof Error ? e.message : e), stopAll(), process.exit(1)),
);

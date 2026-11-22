// Screenshot harness: `pnpm -C ui shot <route|url> [--mode mock|real] [--theme light|dark] [--size WxH] [--keys] [--click] [--out]`.
// Writes <out>/<slug>-<theme>-<width>.png, .aria.txt and .console.json; exits 1 on any console error or failed request.
import { type ChildProcess, spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from '@playwright/test';

const uiDir = resolve(import.meta.dirname, '..');
const repoDir = resolve(uiDir, '..');

const up = (url: string, init?: RequestInit) =>
  fetch(url, init).then(() => true, () => false);

async function waitFor(url: string, init?: RequestInit) {
  for (let i = 0; i < 300; i++) {
    if (await up(url, init)) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`timed out waiting for ${url}`);
}

/** A port nobody holds right now, so parallel runs do not collide. */
const freePort = () =>
  new Promise<number>((ok, fail) => {
    const s = createServer().once('error', fail).listen(0, '127.0.0.1', () => {
      const { port } = s.address() as { port: number };
      s.close(() => ok(port));
    });
  });

const started: ChildProcess[] = [];
function launch(cmd: string, args: string[], cwd: string, env: Record<string, string> = {}) {
  // Own process group so the whole tree (pnpm, vite, cargo) can be stopped.
  const p = spawn(cmd, args, { cwd, env: { ...process.env, ...env }, stdio: ['ignore', 'pipe', 'inherit'], detached: true });
  started.push(p);
  return p;
}
function stopAll() {
  for (const p of started) if (p.pid) try { process.kill(-p.pid); } catch { /* already gone */ }
}

/** Seeds a throwaway db and serves it on a free port; resolves to the devserver's base url. */
async function startDevserver(origin: string) {
  const db = join(mkdtempSync(join(tmpdir(), 'taskapp-shot-')), 'shot.db');
  const run = (sub: string[]) => ['run', '-q', '-p', 'taskapp-devserver', '--', ...sub, '--db', db];
  await new Promise<void>((ok, fail) =>
    spawn('cargo', run(['seed']), { cwd: repoDir, stdio: 'inherit' }).on('exit', (c) =>
      c === 0 ? ok() : fail(new Error('seed failed')),
    ),
  );
  const p = launch('cargo', [...run(['serve', '--port', '0', '--allow-origin', origin])], repoDir);
  return new Promise<string>((ok, fail) => {
    p.once('exit', () => fail(new Error('devserver exited before listening')));
    p.stdout!.on('data', (d: Buffer) => {
      const m = /listening on (\S+)/.exec(String(d));
      if (m) ok(`http://${m[1]}`);
    });
  });
}

async function ensureServers(mode: 'mock' | 'real') {
  // The probe frees the port before vite binds it, so a parallel run can take it in between. --strictPort then
  // makes vite exit instead of moving on; that exit, or no "Local:" line, means retry with a new port. The line
  // also proves the server answering on the port is ours, not another run's.
  for (let attempt = 1; ; attempt++) {
    const vite = await freePort();
    const base = `http://localhost:${vite}`;
    const env: Record<string, string> = { VITE_API_MODE: mode === 'mock' ? 'mock' : 'http' };
    if (mode === 'real') env.VITE_API_URL = await startDevserver(base);
    const p = launch('pnpm', ['exec', 'vite', '--host', 'localhost', '--port', String(vite), '--strictPort'], uiDir, env);
    // Ready when vite prints its URL; if the line is never seen (a different log format), a server that answers
    // while our process is still alive after a few seconds counts too (a lost port makes vite exit by then).
    let seen = false;
    p.stdout!.on('data', (d: Buffer) => void (seen ||= /localhost:\d+/.test(String(d))));
    const t0 = Date.now();
    while (p.exitCode === null && p.signalCode === null) {
      if (seen || (Date.now() - t0 > 8000 && (await up(base)))) return base;
      if (Date.now() - t0 > 120_000) throw new Error(`vite not ready on ${base}`);
      await new Promise((r) => setTimeout(r, 200));
    }
    if (attempt === 5) throw new Error(`vite did not start on a free port in ${attempt} tries`);
  }
}

const slug = (route: string) => route.replace(/^https?:\/\//, '').replace(/[^a-zA-Z0-9]+/g, '-').replace(/^-|-$/g, '') || 'root';
const isKeyName = (t: string) => /^[A-Z][A-Za-z0-9+]+$/.test(t);

async function main() {
  const { values, positionals } = parseArgs({
    allowPositionals: true,
    options: {
      mode: { type: 'string', default: 'mock' },
      theme: { type: 'string', default: 'light' },
      size: { type: 'string', default: '1440x900' },
      keys: { type: 'string' },
      click: { type: 'string' },
      out: { type: 'string', default: join(uiDir, 'harness-out') },
    },
  });
  const route = positionals[0];
  const size = /^(\d+)x(\d+)$/.exec(values.size);
  if (!route || (values.mode !== 'mock' && values.mode !== 'real') || !size || !['light', 'dark'].includes(values.theme)) {
    throw new Error('usage: shot <route|url> [--mode mock|real] [--theme light|dark] [--size 1440x900|1024x768] [--keys "a,b,Enter"] [--click selector]');
  }
  const [, width, height] = size;
  // A full URL is opened as is, with no dev server started.
  const url = /^https?:\/\//.test(route) ? route : (await ensureServers(values.mode)) + route;

  const problems: { type: string; text: string }[] = [];
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: Number(width), height: Number(height) }, colorScheme: values.theme as 'light' | 'dark' });
    // The browser asks for /favicon.ico on its own; the app has none.
    const noise = (u: string) => u.endsWith('/favicon.ico');
    page.on('console', (m) => m.type() === 'error' && problems.push({ type: 'console.error', text: m.text() }));
    page.on('pageerror', (e) => problems.push({ type: 'pageerror', text: e.message }));
    page.on('requestfailed', (r) => noise(r.url()) || problems.push({ type: 'requestfailed', text: `${r.url()} ${r.failure()?.errorText}` }));
    page.on('response', (r) => r.status() >= 400 && !noise(r.url()) && problems.push({ type: 'http', text: `${r.status()} ${r.url()}` }));

    await page.goto(url, { waitUntil: 'networkidle' });
    // Apps that theme by attribute rather than by media query follow too.
    await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), values.theme);
    if (values.click) await page.click(values.click);
    for (const token of (values.keys ?? '').split(',').filter(Boolean)) {
      if (isKeyName(token)) await page.keyboard.press(token);
      else await page.keyboard.type(token);
    }
    await page.waitForLoadState('networkidle');

    mkdirSync(values.out, { recursive: true });
    const base = join(values.out, `${slug(route)}-${values.theme}-${width}`);
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

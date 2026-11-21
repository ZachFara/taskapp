import { execFile, execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { createServer, type Server } from 'node:http';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const uiDir = resolve(import.meta.dirname, '..');
// Async, so this process can still answer the throwaway page server while the shot runs.
const shot = (args: string[]) =>
  new Promise<{ status: number; stderr: string }>((done) =>
    execFile('pnpm', ['exec', 'tsx', 'harness/shot.ts', ...args], { cwd: uiDir }, (err, _out, stderr) =>
      done({ status: err ? Number(err.code) : 0, stderr }),
    ),
  );
const outDir = () => mkdtempSync(join(tmpdir(), 'shot-test-'));

const servers: Server[] = [];
afterAll(() => servers.forEach((s) => s.close()));
/** A throwaway page running `script`. */
async function page(script: string): Promise<string> {
  const s = createServer((req, res) => {
    if (req.url === '/missing') return void res.writeHead(404).end();
    res.writeHead(200, { 'content-type': 'text/html' }).end(`<h1>t</h1><script>${script}</script>`);
  });
  servers.push(s);
  await new Promise<void>((ok) => s.listen(0, '127.0.0.1', ok));
  return `http://127.0.0.1:${(s.address() as { port: number }).port}/`;
}

describe('shot', () => {
  // Idempotent: a no-op when the browser is cached, so CI needs no separate install step.
  beforeAll(() => void execFileSync('pnpm', ['exec', 'playwright', 'install', 'chromium'], { cwd: uiDir }), 300_000);

  it('mock_mode_writes_png_aria_and_console', async () => {
    const out = outDir();
    const r = await shot(['/view/1', '--mode', 'mock', '--out', out]);
    expect(r.status, r.stderr).toBe(0);
    expect(statSync(join(out, 'view-1.png')).size).toBeGreaterThan(0);
    expect(readFileSync(join(out, 'view-1.aria.txt'), 'utf8')).toContain('ListView'); // placeholder page: change to 'ZF-1' once the list renders issues
    expect(JSON.parse(readFileSync(join(out, 'view-1.console.json'), 'utf8'))).toEqual([]);
  }, 120_000);

  it('console_error_exits_nonzero', async () => {
    const out = outDir();
    const r = await shot([await page('console.error("boom")'), '--out', out]);
    expect(r.status).toBe(1);
    const log = readdirSync(out).find((f) => f.endsWith('.console.json'))!;
    expect(readFileSync(join(out, log), 'utf8')).toContain('boom');
  }, 60_000);

  it('failed_network_request_exits_nonzero', async () => {
    const out = outDir();
    const r = await shot([await page('fetch("/missing")'), '--out', out]);
    expect(r.status).toBe(1);
  }, 60_000);

  it('clean_page_exits_zero', async () => {
    const r = await shot([await page('0'), '--out', outDir()]);
    expect(r.status, r.stderr).toBe(0);
  }, 60_000);
});

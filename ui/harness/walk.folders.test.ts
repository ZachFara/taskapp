import { type ChildProcess, execFileSync, spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { resolve } from 'node:path';
import { chromium, expect as pw, type Browser, type Page } from '@playwright/test';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

// The first-minute flows, driven as a user would in the mock app, asserting on what is on screen.
const uiDir = resolve(import.meta.dirname, '..');
let vite: ChildProcess;
let browser: Browser;
let base = '';
let page: Page;

const freePort = () =>
  new Promise<number>((ok, fail) => {
    const s = createServer().once('error', fail).listen(0, '127.0.0.1', () => {
      const { port } = s.address() as { port: number };
      s.close(() => ok(port));
    });
  });

beforeAll(async () => {
  execFileSync('pnpm', ['exec', 'playwright', 'install', 'chromium'], { cwd: uiDir });
  const port = await freePort();
  base = `http://localhost:${port}`;
  vite = spawn('pnpm', ['exec', 'vite', '--host', 'localhost', '--port', String(port), '--strictPort'], {
    cwd: uiDir, env: { ...process.env, VITE_API_MODE: 'mock' }, stdio: 'ignore', detached: true,
  });
  let up = false;
  for (let i = 0; i < 300 && !up; i++) {
    up = await fetch(base).then(() => true, () => false);
    if (!up) await new Promise((r) => setTimeout(r, 200));
  }
  if (!up) throw new Error(`vite did not answer on ${base}`);
  browser = await chromium.launch();
  // The first load compiles every module, which is slower than the 5s page timeout the tests use; pay that here.
  const warm = await browser.newPage();
  await warm.goto(base, { timeout: 90_000 });
  await warm.close();
}, 180_000);

afterAll(async () => {
  await browser?.close();
  if (vite?.pid) try { process.kill(-vite.pid); } catch { /* already gone */ }
});

async function open(path = '/view/1', showProjects = true) {
  await page?.close();
  page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  // The sidebar lists every project only when the device switch is on.
  await page.addInitScript((on) => on && localStorage.setItem('sidebar.showProjects', '1'), showProjects);
  page.setDefaultTimeout(Number(process.env.WALK_TIMEOUT ?? 5_000)); // WALK_TIMEOUT: more time on a loaded machine
  await page.goto(base + path);
  await page.locator('aside.sidebar').waitFor();
}
const side = () => page.locator('aside.sidebar');
const row = (text: string, section = '') => side().locator(section ? `[aria-label=${section}] li.row` : 'li.row', { hasText: text }).first();

// Real pointer input: press on a row, move over the target, release.
async function drag(from: string, to: string, section: string) {
  const [a, b] = [(await row(from, section).boundingBox())!, (await row(to, section).boundingBox())!];
  await page.mouse.move(a.x + 30, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(a.x + 40, a.y + a.height / 2 + 6, { steps: 3 });
  await page.mouse.move(b.x + 40, b.y + b.height / 2, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(200);
}
async function newFolder(section: string, name: string) {
  await side().getByRole('button', { name: `${section} menu` }).click();
  await page.getByRole('menuitem', { name: 'New folder' }).click();
  await page.getByRole('textbox', { name: 'Name' }).fill(name);
  await page.getByRole('dialog').getByRole('button', { name: 'Rename', exact: true }).click();
  await page.waitForTimeout(150);
}
const depth = (text: string, section: string) => row(text, section).evaluate((e) => (e as HTMLElement).style.getPropertyValue('--depth'));

describe('walk folders', { timeout: 40_000 }, () => {
  it('a view and a folder are dragged into folders, a project is moved with the menu', async () => {
    await open();
    await newFolder('Views', 'Work');
    await newFolder('Views', 'Inner');
    await drag('Board', 'Work', 'Views');
    expect(await depth('Board', 'Views')).toBe('1');
    await drag('Inner', 'Work', 'Views');
    expect(await depth('Inner', 'Views')).toBe('1');
    // Esc cancels a drag: nothing moves
    const [a, b] = [(await row('Active', 'Views').boundingBox())!, (await row('Work', 'Views').boundingBox())!];
    await page.mouse.move(a.x + 30, a.y + a.height / 2);
    await page.mouse.down();
    await page.mouse.move(b.x + 40, b.y + b.height / 2, { steps: 8 });
    await page.keyboard.press('Escape');
    await page.mouse.up();
    expect(await depth('Active', 'Views')).toBe('0');

    await newFolder('Projects', 'Live');
    await row('Website relaunch', 'Projects').click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Move to folder' }).hover();
    await page.getByRole('menuitem', { name: 'Live' }).click();
    await page.waitForTimeout(200);
    expect(await depth('Website relaunch', 'Projects')).toBe('1');
  });

  it('a folder and a project view are pinned, and the project view is dragged into a view folder', async () => {
    await open('/views', false);
    await pw(side().locator('[aria-label=Projects] li.row')).toHaveCount(1); // only "All projects"
    await page.getByRole('button', { name: 'New view' }).first().click();
    await page.getByRole('tab', { name: 'Projects' }).click();
    await page.getByRole('button', { name: 'Create view' }).click();
    await page.waitForURL(/\/view\/\d+/);
    const name = (await page.locator('h1').first().textContent())!.trim();
    await newFolder('Views', 'Work');
    await row('Work', 'Views').click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'Pin Work' }).click();
    await row(name, 'Views').click({ button: 'right' });
    await page.getByRole('menuitem', { name: `Pin ${name}` }).click();
    await pw(row('Work', 'Pinned')).toBeVisible();
    await pw(row(name, 'Pinned').first()).toBeVisible();
    await drag(name, 'Work', 'Views');
    expect(await depth(name, 'Views')).toBe('1');
    // the pinned folder shows it too, one step in
    await pw(side().locator('[aria-label=Pinned] li.row', { hasText: name })).toHaveCount(2);
    await page.screenshot({ path: resolve(uiDir, '../.scratch/sidebar2-walk.png') });
  });
});

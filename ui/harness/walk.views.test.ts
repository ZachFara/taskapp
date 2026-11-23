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
  for (let i = 0; i < 300; i++) {
    if (await fetch(base).then(() => true, () => false)) break;
    await new Promise((r) => setTimeout(r, 200));
  }
  browser = await chromium.launch();
}, 120_000);

afterAll(async () => {
  await browser?.close();
  if (vite?.pid) try { process.kill(-vite.pid); } catch { /* already gone */ }
});

async function open(path = '/view/1', viewport = { width: 1440, height: 900 }) {
  await page?.close();
  page = await browser.newPage({ viewport });
  page.setDefaultTimeout(5_000);
  await page.goto(base + path);
  await page.locator('[data-key], aside.sidebar').first().waitFor();
  if (path.startsWith('/view/')) await page.locator('li[data-key], li[data-issue], tr[data-key]').first().waitFor();
}
const press = async (...keys: string[]) => {
  for (const k of keys) await page.keyboard.press(k);
};
const rows = () => page.locator('li[data-key]');
const focusedKey = () => page.locator('li[data-key][aria-current=true], [data-focused]').first().getAttribute('data-key');
const palette = () => page.getByRole('dialog', { name: 'Command menu' });
const statusOf = (key: string) => rows().filter({ has: page.locator(`.key:text-is("${key}")`) }).getByRole('button', { name: /^Status/ }).getAttribute('aria-label');


describe('walk views', { timeout: 25_000 }, () => {
  it('R3-1 clicking Add a description focuses the textarea and ? types a ?', async () => {
    await open('/issue/ABC-2');
    await page.getByRole('button', { name: 'Edit description' }).click();
    await page.waitForTimeout(200);
    expect(await page.evaluate(() => document.activeElement?.tagName)).toBe('TEXTAREA');
    await page.keyboard.type('a?');
    expect(await page.getByLabel('Description text').inputValue()).toContain('a?');
    expect(await page.getByRole('dialog', { name: /shortcuts/i }).count()).toBe(0);
  });

  it('R3-9 cmd-click selects without opening the panel and sets aria-selected', async () => {
    await open();
    await rows().nth(0).click({ modifiers: ['Meta'] });
    await rows().nth(1).click({ modifiers: ['Meta'] });
    await page.waitForTimeout(200);
    expect(await page.locator('aside.panel').count()).toBe(0);
    expect(await page.locator('li[data-key][aria-selected=true]').count()).toBe(2);
    expect(await page.locator('li[data-key][aria-selected=false]').count()).toBeGreaterThan(0);
  });

  it('R4-09 quick add: p1 is Urgent and the due chip is a human date', async () => {
    await open();
    await press('c');
    await page.getByLabel('Title, with quick-add tokens').fill('Ship it p1 due 2026-11-27');
    await page.waitForTimeout(300);
    expect(await page.locator('[data-chip][data-kind=priority]').getAttribute('aria-label')).toContain('Urgent');
    expect(await page.locator('[data-chip][data-kind=due]').innerText()).toContain('Nov 27');
  });

  it('R4-05 side panel order: description before the properties block', async () => {
    await open();
    await rows().first().click();
    await press('Enter');
    await page.locator('aside.panel [aria-label=Description]').waitFor();
    const d = await page.locator('aside.panel section[aria-label=Description]').boundingBox();
    const p = await page.locator('aside.panel [aria-label=Details]').boundingBox();
    expect(d!.y).toBeLessThan(p!.y);
  });

  it('R4-01 1024 with the panel open keeps the title column and the header inside the sheet', async () => {
    await open('/view/1', { width: 1024, height: 768 });
    await rows().first().click();
    await press('Enter');
    await page.locator('aside.panel').waitFor();
    await page.waitForTimeout(300);
    const t = await rows().first().locator('.title .text').first().boundingBox();
    expect(t!.width).toBeGreaterThan(120);
    const close = await page.locator('aside.panel').getByRole('button', { name: 'Close issue' }).boundingBox();
    expect(close!.x + close!.width).toBeLessThanOrEqual(1024);
    const key = await page.locator('aside.panel .key').boundingBox();
    expect(key!.height).toBeLessThan(30);
  });

  it('R4-02 table at 1024 scrolls inside itself with the title column sticky', async () => {
    await open('/view/1/Table', { width: 1024, height: 768 });
    const sc = page.locator('main.body');
    expect(await sc.evaluate((e) => e.scrollWidth > e.clientWidth)).toBe(true);
    expect(await page.locator('th[data-sticky], td[data-sticky]').count()).toBeGreaterThan(0);
  });

  it('R3-4 Sort, Display, row status, filter remove, group add and view tabs show a tooltip with the shortcut', async () => {
    await open();
    const tip = async (loc: import('@playwright/test').Locator, text: RegExp) => {
      await page.mouse.move(0, 0);
      await page.waitForTimeout(150);
      await loc.hover();
      await page.locator('.cx-tip').first().waitFor({ timeout: 2500 });
      expect(await page.locator('.cx-tip').first().innerText()).toMatch(text);
    };
    await tip(page.getByRole('button', { name: /^Sort/ }), /Sort/);
    await tip(page.getByRole('button', { name: /^Display/ }), /Display/);
    await tip(page.getByRole('button', { name: /^Group/ }), /Group/);
    await tip(rows().first().getByRole('button', { name: /^Status/ }), /Status/);
    await tip(page.getByRole('button', { name: /^Remove filter/ }).first(), /Remove filter/);
  });

  it('events: description.edit focuses the editor; view.rename opens the rename form', async () => {
    await open('/issue/ABC-2');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:issue.description.edit')));
    await page.waitForTimeout(200);
    expect(await page.evaluate(() => document.activeElement?.tagName)).toBe('TEXTAREA');
    for (const [ev, sel] of [['issue.comment.focus', 'TEXTAREA'], ['issue.subissue.add', 'INPUT'], ['issue.relation.add', 'INPUT']]) {
      await page.evaluate((e) => dispatchEvent(new CustomEvent(`command:${e}`)), ev);
      await page.waitForTimeout(150);
      expect(await page.evaluate(() => document.activeElement?.tagName), ev).toBe(sel);
    }
    await open();
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:view.rename')));
    await page.getByLabel('View name').waitFor();
  });
});

describe('walk groups', { timeout: 25_000 }, () => {
  it('group.collapse hides the focused row group; group.collapse.all and group.add act on the list', async () => {
    await open();
    const before = await rows().count();
    await press('j');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBeLessThan(before);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse.all')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBe(0);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse.all')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBe(before);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.add')));
    await page.waitForTimeout(300);
    expect(await rows().count()).toBe(before + 1);
  });
});

describe('walk project', { timeout: 25_000 }, () => {
  it('project and milestone events open their editors', async () => {
    await open('/project/website');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:project.rename')));
    await page.getByLabel('Project name').waitFor();
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:milestone.new')));
    await page.getByLabel('Milestone name').waitFor();
  });
});

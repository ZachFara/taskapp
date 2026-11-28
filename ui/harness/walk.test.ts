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

async function open(path = '/view/1', viewport = { width: 1440, height: 900 }) {
  await page?.close();
  page = await browser.newPage({ viewport });
  page.setDefaultTimeout(Number(process.env.WALK_TIMEOUT ?? 5_000)); // WALK_TIMEOUT: more time on a loaded machine
  await page.goto(base + path);
  await page.locator('[data-key], aside.sidebar').first().waitFor();
  if (path.startsWith('/view/')) await page.locator('li[data-key], li[data-issue]').first().waitFor();
}
const press = async (...keys: string[]) => {
  for (const k of keys) await page.keyboard.press(k);
};
const rows = () => page.locator('li[data-key]');
const focusedKey = () => page.locator('li[data-key][aria-current=true], [data-focused]').first().getAttribute('data-key');
const palette = () => page.getByRole('dialog', { name: 'Command menu' });
const statusOf = (key: string) => rows().filter({ has: page.locator(`.key:text-is("${key}")`) }).getByRole('button', { name: /^Status/ }).getAttribute('aria-label');

describe('walk', { timeout: 20_000 }, () => {
  it('F1 Esc with nothing open stays in the app', async () => {
    await open();
    await press('Escape', 'Escape', 'Escape');
    await page.waitForTimeout(300);
    expect(new URL(page.url()).pathname).toBe('/view/1');
    expect(await rows().count()).toBeGreaterThan(0);
  });

  it('F1 Esc on an issue page returns to the list it came from', async () => {
    await open();
    await press('j', 'Meta+Enter');
    await page.waitForURL(/\/issue\/ABC-/);
    await page.waitForTimeout(250);
    await press('Escape');
    await page.waitForURL(/\/view\/1$/);
  });

  it('F2 s opens the status page; typing prog sets the whole selection; u undoes both', async () => {
    await open();
    const [a, b] = await rows().evaluateAll((l) => l.slice(0, 2).map((x) => x.getAttribute('data-key')!));
    const before = [await statusOf(a), await statusOf(b)];
    await press('j', 'x', 'j', 'x', 's');
    await pw(palette().getByText('Status', { exact: true }).first()).toBeVisible();
    await pw(palette().getByRole('option', { name: 'In Progress' })).toBeVisible();
    expect(await palette().getByRole('combobox').inputValue()).toBe('');
    await page.keyboard.type('prog');
    await press('Enter');
    await pw(palette()).toHaveCount(0);
    await expect.poll(() => statusOf(a)).toContain('In Progress');
    await expect.poll(() => statusOf(b)).toContain('In Progress');
    await press('u');
    await expect.poll(() => statusOf(a)).toBe(before[0]);
    await expect.poll(() => statusOf(b)).toBe(before[1]);
  });

  it('F2 d then "next fri" shows the parsed date first and Enter sets it', async () => {
    await open();
    await press('j', 'j', 'd'); // ABC-6, due Nov 26
    await pw(palette().getByRole('option', { name: 'Today' })).toBeVisible();
    await page.keyboard.type('next fri');
    const first = palette().getByRole('option').first();
    await pw(first).toContainText(/Set the due date to/);
    const day = /(?:Nov|Dec|Jan) \d+/.exec((await first.innerText()))![0];
    await press('Enter');
    await pw(palette()).toHaveCount(0);
    await pw(rows().filter({ hasText: 'ABC-6' })).toContainText(day);
  });

  it('F3 palette root and right-click menu show only what applies here', async () => {
    await open();
    await press('Meta+k');
    const labels = await palette().getByRole('option').allTextContents();
    expect(labels.length).toBeGreaterThan(5);
    expect(labels.join('|')).not.toMatch(/Month view|Edit cell|Week view|Zoom/);
    await press('Escape');
    await rows().nth(2).click({ button: 'right' });
    const items = page.getByRole('menuitem');
    await items.first().waitFor();
    expect(await items.count()).toBeLessThanOrEqual(19);
    const top = await page.getByRole('menu').evaluate((m) => m.getBoundingClientRect().top);
    expect(top).toBeGreaterThanOrEqual(0);
  });

  it('F4 on the board j then l moves to the next column', async () => {
    await open('/view/2');
        await press('j');
    const col = () => page.locator('[data-focused]').first().evaluate((c) => [...document.querySelectorAll('ul.cards')].indexOf(c.closest('ul.cards')!));
    const before = await col();
    await press('l');
    await pw(palette()).toHaveCount(0);
    expect(await col()).toBeGreaterThan(before);
  });

  it('F5 Enter opens the side panel and keeps the list; j follows; Esc closes it', async () => {
    await open();
    await press('j', 'Enter');
    await page.waitForTimeout(300);
    expect(new URL(page.url()).pathname).toBe('/view/1');
    const panel = page.locator('aside.panel');
    await pw(panel).toBeVisible();
    const first = await focusedKey();
    await pw(panel).toContainText(first!);
    await press('j');
    const next = await focusedKey();
    expect(next).not.toBe(first);
    await pw(panel).toContainText(next!);
    await page.waitForTimeout(250); // the panel ignores an Esc that lands within 60ms of a reload
    await press('Escape');
    await pw(panel).toHaveCount(0);
  });

  it('M1 c then typing at once: every character lands in the title while the dialog arrives', async () => {
    await open();
    await press('c');
    await page.keyboard.type('Quick motion title');
    await pw(page.getByLabel('Title, with quick-add tokens')).toHaveValue('Quick motion title');
  });

  it('F6 the created toast has a readable Undo; u with nothing to undo is quiet', async () => {
    await open();
    await press('c');
    await page.keyboard.type('Walk issue');
    await press('Enter');
    // Toasts stack, so the one just made is the newest (the last).
    const undo = page.getByRole('status').getByRole('button', { name: /Undo/ }).last();
    await pw(undo).toBeVisible();
    const [fg, bg] = await undo.evaluate((b) => [getComputedStyle(b).color, getComputedStyle(b.closest('[role=status]')!).backgroundColor]);
    expect(fg).not.toBe(bg);
    await open();
    await press('u');
    const toast = page.getByRole('status').last();
    await pw(toast).toContainText('Nothing to undo');
    await pw(toast).not.toContainText('invalid');
  });

  it('F8/F9 the cheat sheet has no duplicates and the command is New issue', async () => {
    await open();
    await press('?');
    const sheet = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    const names = await sheet.locator('li[data-row] .name').allTextContents();
    expect(names.filter((n, i) => names.indexOf(n) !== i)).toEqual([]);
    const heads = await sheet.locator('h3').allTextContents();
    expect(heads.filter((h) => /^Navigat/.test(h)).length).toBe(1);
    expect(names).toContain('New issue');
    expect(names).not.toContain('Create issue');
  });

  it('F10 / searches issues by the start of a word; Enter opens the issue', async () => {
    await open();
    await press('/');
    await pw(palette().getByRole('combobox')).toHaveAttribute('aria-label', 'Search issues');
    await page.keyboard.type('offl');
    await pw(palette().getByRole('option').first()).toContainText('Offline mode research');
    await press('Enter');
    await page.waitForURL(/\/issue\/ABC-/);
  });

  it('5 j moves five rows', async () => {
    await open();
    await press('5', 'j');
    expect(await rows().evaluateAll((l) => l.findIndex((x) => x.getAttribute('aria-current') === 'true'))).toBe(4);
  });

  it('W1 a custom property switched on in Display shows in the list, board and table', async () => {
    await open();
    await page.getByRole('button', { name: 'Display' }).click();
    const panel = page.getByRole('group', { name: 'Display options' });
    await pw(panel.getByRole('switch', { name: 'Show Area' })).toBeVisible();
    await panel.getByRole('switch', { name: 'Show Area' }).click();
    await pw(page.locator('li[data-key="ABC-1"] [data-field="P:area"]')).toContainText('work');
    await press('Escape');
    await page.waitForTimeout(600); // the debounced save
    await page.getByRole('tab', { name: 'Board' }).click();
    await pw(page.locator('[data-issue="ABC-1"]')).toContainText('work');
    await page.getByRole('tab', { name: 'Table' }).click();
    await pw(page.locator('tr[data-key="ABC-1"]')).toContainText('work');
  });

  it('W2 the Views page lists the seeded templates; New view from one; Save as template; Reset', async () => {
    await open('/views');
    for (const n of ['Status board', 'Planning table', 'This week']) await pw(page.getByText(n, { exact: true }).first()).toBeVisible();
    await page.getByRole('button', { name: 'New view' }).click();
    const dialog = page.getByRole('dialog', { name: 'New view' });
    await dialog.getByRole('button', { name: /Status board/ }).click();
    await dialog.getByRole('button', { name: 'Create view' }).click();
    await page.waitForURL(/\/view\/\d+$/);
    await pw(page.getByRole('tab', { name: 'Board' })).toHaveAttribute('aria-selected', 'true');
    // Away from the template and back.
    await page.getByRole('tab', { name: 'List' }).click();
    await page.waitForTimeout(600);
    await page.getByRole('button', { name: 'More', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Reset to template' }).click();
    await pw(page.getByRole('tab', { name: 'Board' })).toHaveAttribute('aria-selected', 'true');
    await page.getByRole('button', { name: 'More', exact: true }).click();
    await page.getByRole('menuitem', { name: 'New view...' }).click();
    await pw(page.getByRole('dialog', { name: 'New view' }).getByRole('button', { name: /Copy of current view/ })).toBeVisible();
    await press('Escape');
    await page.getByRole('button', { name: 'More', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Save as template' }).click();
    // In-app navigation: a reload would reset the mock.
    await page.evaluate(() => { history.pushState(null, '', '/views'); dispatchEvent(new PopStateEvent('popstate')); });
    await pw(page.getByRole('main').getByText('Status board', { exact: true })).toHaveCount(3); // the new view and two templates
  });

  it('W8 at 1024 wide the layout tabs are icons and New issue is not clipped', async () => {
    await open('/view/1', { width: 1024, height: 768 });
    await pw(page.locator('.cx-tab-label').first()).toBeHidden();
    await pw(page.getByRole('tab', { name: 'Board' })).toBeVisible();
    const box = await page.getByRole('button', { name: 'New issue' }).first().boundingBox();
    expect(box!.x + box!.width).toBeLessThanOrEqual(1024);
  });

  it('X1 shift+i opens the insights panel with a chart and headline numbers; Measure to issues; close', async () => {
    await open();
    await press('Shift+I');
    const panel = page.getByRole('complementary', { name: 'Insights' });
    await pw(panel).toBeVisible();
    await pw(panel.locator('path.scope').first()).toHaveAttribute('d', /^M/);
    expect(await panel.locator('[data-headline] dd').allTextContents()).toSatisfy((n: string[]) => n.length >= 3 && n.every((t) => t.trim() !== ''));
    await pw(panel.locator('svg[role=img]').first()).toHaveAttribute('aria-label', /points/);
    await panel.getByRole('combobox', { name: 'Measure' }).click();
    await page.getByRole('option', { name: 'Issues' }).click();
    await pw(panel.locator('svg[role=img]').first()).toHaveAttribute('aria-label', /issues/);
    await panel.getByRole('button', { name: 'Close insights' }).click();
    await pw(panel).toBeHidden();
  });

  it('FX2 the cheat sheet fits the window, scrolls inside, and shows keys as glyphs', async () => {
    await open('/view/1', { width: 1024, height: 768 });
    await press('?');
    const dialog = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
    await pw(dialog).toBeVisible();
    const box = (await dialog.boundingBox())!;
    expect(box.y + box.height).toBeLessThanOrEqual(768);
    const scroll = dialog.locator('.scroll');
    expect(await scroll.evaluate((e) => e.scrollHeight > e.clientHeight)).toBe(true);
    await scroll.evaluate((e) => e.scrollTo(0, e.scrollHeight));
    await pw(dialog.getByRole('searchbox', { name: 'Filter shortcuts' })).toBeInViewport();
    const text = (await dialog.textContent())!;
    expect(text).toMatch(/⌘|Ctrl\+/); // the browser's own platform decides
    expect(text).not.toMatch(/mod\+|arrowdown|escape/);
  });

  it('FX2 shortcuts do not fire behind an open menu', async () => {
    await open();
    await page.getByRole('button', { name: 'Add filter' }).click();
    await pw(page.getByRole('menu')).toBeVisible();
    await press('s');
    await page.waitForTimeout(300);
    await pw(palette()).toBeHidden();
  });

  // In-app navigation, so the mock keeps what the test just wrote.
  const go = async (path: string) => {
    await page.evaluate((p) => { history.pushState(null, '', p); dispatchEvent(new PopStateEvent('popstate')); }, path);
    await page.waitForTimeout(400);
  };

  it('P3 an event is created, linked to an issue, unlinked, and the unlink is undone', async () => {
    await open('/events');
    await page.getByRole('button', { name: 'New event' }).first().click();
    await page.getByLabel('Event title').fill('Launch review');
    await page.getByLabel('Date', { exact: true }).fill('2026-11-25');
    await page.getByRole('checkbox', { name: 'All day' }).click();
    await page.getByLabel('From').fill('10:00');
    await page.getByLabel('To', { exact: true }).fill('11:30');
    await page.getByRole('button', { name: 'Create event' }).click();
    await page.waitForURL(/\/event\/\d+/);
    await page.getByLabel('Link an issue').fill('Redesign');
    await page.locator('.results button', { hasText: 'ABC-1' }).click();
    await pw(page.getByRole('button', { name: 'Unlink ABC-1' })).toBeVisible();
    const eventUrl = new URL(page.url()).pathname;
    await go('/issue/ABC-1');
    await pw(page.locator('section[aria-label=Events]')).toContainText('Launch review');
    await go(eventUrl);
    await page.getByRole('button', { name: 'Unlink ABC-1' }).click();
    await pw(page.getByRole('button', { name: 'Unlink ABC-1' })).toHaveCount(0);
    await press('Meta+z');
    await pw(page.getByRole('button', { name: 'Unlink ABC-1' })).toBeVisible(); // the page reloads on undo
  });

  it('P3 a custom relation kind reads both ways and cannot be deleted while used', async () => {
    await open('/settings/relations');
    await page.getByLabel('Kind name').fill('Mentors');
    await page.getByLabel('Forward label').fill('mentors');
    await page.getByLabel('Reverse label').fill('is mentored by');
    await page.getByRole('button', { name: 'Add kind' }).click();
    await go('/issue/ABC-1');
    await page.getByLabel('Relation kind').click();
    await page.getByRole('option', { name: 'mentors', exact: true }).click();
    await page.getByLabel('Find issue').fill('Migrate');
    await page.locator('.results button', { hasText: 'ABC-4' }).click();
    await go('/issue/ABC-4');
    await pw(page.getByText('is mentored by').first()).toBeVisible();
    await go('/settings/relations');
    await page.getByRole('button', { name: 'Delete Mentors' }).click();
    await page.getByRole('button', { name: 'Delete kind' }).click();
    await pw(page.getByRole('alert')).toContainText('Mentors');
    await pw(page.getByRole('button', { name: 'Delete kind' })).toHaveCount(0); // the dialog closed, the message shows
  });

  it('P3 a field switched off disappears from the issue panel and filters, and comes back', async () => {
    await open('/settings/fields');
    await page.getByLabel('Show Due').click();
    await go('/issue/ABC-1');
    await pw(page.getByText('Estimate', { exact: true })).toBeVisible();
    expect(await page.getByText('Due', { exact: true }).count()).toBe(0);
    await go('/view/1');
    await page.getByRole('button', { name: 'Add filter' }).click();
    await pw(page.getByRole('menuitem', { name: 'Project', exact: true })).toBeVisible();
    expect(await page.getByRole('menuitem', { name: 'Due', exact: true }).count()).toBe(0);
    await press('Escape');
    await go('/settings/fields');
    await page.getByLabel('Show Due').click();
    await go('/issue/ABC-1');
    await pw(page.getByText('Due', { exact: true })).toBeVisible();
  });
  it('IMAGES pasting an image into a description shows it in the description', async () => {
    await open('/issue/ABC-1');
    await page.getByRole('button', { name: 'Edit description' }).click();
    const box = page.getByRole('textbox', { name: 'Description text' });
    await box.focus();
    // A real 1x1 PNG, pasted as the clipboard would carry it.
    await box.evaluate((el) => {
      const png = Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg=='), (c) => c.charCodeAt(0));
      const data = new DataTransfer();
      data.items.add(new File([png], 'shot.png', { type: 'image/png' }));
      el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
    });
    await pw(box).toHaveValue(/!\[shot\]\(attachment:\d+\)/);
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    const img = page.getByRole('region', { name: 'Description' }).locator('img[data-attachment]');
    await pw(img).toHaveAttribute('src', /^blob:/);
    await page.waitForFunction(() => (document.querySelector('img[data-attachment]') as HTMLImageElement)?.naturalWidth > 0);
    await img.click();
    await pw(page.getByRole('dialog', { name: 'Image' }).locator('img')).toBeVisible();
  });

  it('DASH the dashboard opens from the sidebar, a card hides, and the Settings switch removes the page', async () => {
    await open('/my');
    await page.getByRole('link', { name: 'Dashboard' }).click();
    await pw(page.getByRole('region', { name: 'Summary' })).toBeVisible();
    await pw(page.getByRole('region', { name: 'Burn-up' })).toBeVisible();
    await pw(page.getByRole('region', { name: 'Stale' })).toBeVisible();
    await page.getByRole('button', { name: 'Cards' }).click();
    await page.getByRole('checkbox', { name: 'Burn-up' }).click();
    await pw(page.getByRole('region', { name: 'Burn-up' })).toHaveCount(0);
    await open('/settings/features');
    await page.getByRole('switch', { name: 'Dashboard' }).click();
    await pw(page.getByRole('link', { name: 'Dashboard' })).toHaveCount(0);
  });

  it('P5 a next action is set on the project page, listed on Next actions, and stuck projects are marked', async () => {
    await open('/project/mobile');
    await pw(page.getByText('Stuck').first()).toBeVisible();
    await page.getByRole('button', { name: 'Next action', exact: true }).click();
    await page.getByRole('option').nth(1).click();
    await pw(page.getByRole('button', { name: 'Next action', exact: true })).toContainText('ABC-');
    await go('/next');
    await pw(page.locator('tr[data-slug=mobile]')).toContainText('ABC-');
    await pw(page.locator('tr[data-slug=office]')).toContainText('Stuck');
  });

  it('REVIEW the weekly review walks its six steps, records the day, and the Dashboard card shows it', async () => {
    await open('/my');
    await page.getByRole('link', { name: 'Weekly review' }).click();
    await pw(page.getByRole('heading', { name: 'Inbox to zero' })).toBeVisible();
    for (let i = 0; i < 6; i++) {
      await page.getByRole('button', { name: 'Done for this step' }).click();
      if (i < 5) await page.getByRole('button', { name: 'Next', exact: true }).click();
    }
    await page.getByRole('button', { name: 'Finish', exact: true }).click();
    await go('/dashboard');
    await pw(page.getByRole('region', { name: 'Weekly review' })).toContainText('since your weekly review');
  });

  it('PLAN an issue is time-blocked from the form and the block shows planned time', async () => {
    await open('/plan');
    await page.locator('button[data-issue]').first().click();
    await page.getByRole('button', { name: 'Time-block an issue', exact: true }).click();
    await page.getByRole('button', { name: 'Add block' }).click();
    await pw(page.locator('[data-event]')).toHaveCount(1);
    await pw(page.locator('[data-summary]').first()).toContainText('in 1 block');
    await page.getByRole('button', { name: 'Remove time block' }).click();
    await pw(page.locator('[data-event]')).toHaveCount(0);
  });

  it('ARCHIVE an archived issue is deleted for good only after the dialog, and Cancel keeps it', async () => {
    await open();
    await press('j');
    const key = (await focusedKey())!;
    await press('#');
    await page.getByRole('link', { name: 'Archive' }).click();
    const row = page.locator('li.row', { hasText: key });
    await pw(row).toBeVisible();
    await row.getByRole('checkbox').click();
    await page.getByRole('button', { name: 'Delete selected permanently…' }).click();
    const dialog = page.getByRole('dialog', { name: 'Delete 1 issue permanently?' });
    await pw(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Cancel' }).click();
    await pw(dialog).toHaveCount(0);
    await pw(row).toBeVisible();
    await page.getByRole('button', { name: 'Delete selected permanently…' }).click();
    await dialog.getByRole('button', { name: 'Delete 1 issue' }).click();
    await pw(row).toHaveCount(0);
  });

  it('PROCESS the Inbox page opens from the sidebar, a key and a button clear issues, typing never fires a key, and the switch removes it', async () => {
    await open('/my');
    await page.getByRole('link', { name: 'Process Inbox' }).click();
    const title = page.getByRole('textbox', { name: 'Title' });
    await pw(title).toBeVisible();
    const first = await title.inputValue();
    await pw(page.locator('.progress')).toHaveText(/^1 of \d+$/);
    // Typing "o" in the title is text, not "Do it now".
    await title.click();
    await page.keyboard.type('o');
    expect(await title.inputValue()).toBe(first + 'o');
    await title.fill(first);
    await page.locator('.key').click();
    await press('Shift+ArrowRight');
    await pw(page.locator('.progress')).toHaveText(/^2 of \d+$/);
    await press('Shift+ArrowLeft');
    await press('o');
    await pw(title).not.toHaveValue(first);
    await page.getByRole('button', { name: /^Defer/ }).click();
    await page.getByRole('button', { name: 'Next week' }).click();
    await page.locator('form button[type=submit]').click();
    await pw(page.getByRole('alert')).toHaveCount(0);
    await open('/settings/features');
    await page.getByRole('switch', { name: 'Process Inbox' }).click();
    await pw(page.getByRole('link', { name: 'Process Inbox' })).toHaveCount(0);
  });

  it('RECUR the Repeat row sets a rule, shows it in words, offers skip, stops it, and the switch removes the row', async () => {
    await open('/issue/ABC-1');
    await page.getByRole('button', { name: 'Repeat', exact: true }).click();
    await page.getByRole('option', { name: 'Every weekday' }).click();
    await pw(page.getByRole('button', { name: 'Repeat', exact: true })).toHaveText('Every weekday');
    await page.getByRole('button', { name: 'Repeat', exact: true }).click();
    await pw(page.getByRole('button', { name: 'Skip this occurrence' })).toBeVisible();
    await page.getByRole('option', { name: 'Does not repeat' }).click();
    await pw(page.getByRole('button', { name: 'Repeat', exact: true })).toHaveText('None');
    await open('/settings/features');
    await page.getByRole('switch', { name: 'Recurring issues' }).click();
    await go('/issue/ABC-1');
    await pw(page.getByRole('button', { name: 'Repeat', exact: true })).toHaveCount(0);
  });
});

describe('Shutdown ritual', () => {
  it('turns on from the command menu, then finishes the day from the checklist', async () => {
    await open('/my');
    await press('Meta+k');
    await page.keyboard.type('Shutdown ritual');
    await press('Enter');
    await page.getByRole('button', { name: 'Shutdown', exact: true }).click();
    await pw(page.getByRole('dialog')).toContainText('Inbox reviewed');
    await page.getByRole('button', { name: /Finish shutdown/ }).click();
    await pw(page.getByRole('dialog')).toContainText('Shutdown complete');
  });
});

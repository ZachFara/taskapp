import { beforeEach, describe, expect, it } from 'vitest';
import { FEATURES, features, isOn } from '../../lib/features.svelte';
import { listCommands, type Ctx } from '../../lib/commands';
import Features from './Features.svelte';
import { field, mountSettings, toggle } from './testing';

const ctx = { route: '/my', selection: [], focusedIssueId: null, openIssueId: null, navigate: () => {} } as unknown as Ctx;
const ids = () => listCommands(ctx).map((c) => c.id);

beforeEach(() => FEATURES.forEach((f) => (features[f.name] = true)));

describe('Features', () => {
  it('lists every feature with a switch, all on by default but the shutdown ritual', async () => {
    const { target } = await mountSettings(Features);
    for (const f of FEATURES) expect(field(target, f.label).getAttribute('aria-checked')).toBe(f.name === 'shutdown' ? 'false' : 'true');
  });

  it('shows a stored off switch', async () => {
    const { target } = await mountSettings(Features, { settings: { get: async (k: unknown) => (k === 'feature_cycles' ? false : null) } });
    expect(field(target, 'Cycles').getAttribute('aria-checked')).toBe('false');
  });

  it('a switch writes its own key at once, and the commands follow', async () => {
    const { target, api } = await mountSettings(Features);
    expect(ids()).toContain('nav.cycles');
    await toggle(target, 'Cycles');
    expect(await api.settings.get('feature_cycles')).toBe(false);
    expect(isOn('cycles')).toBe(false);
    expect(ids()).not.toContain('nav.cycles');
    expect(ids()).toContain('feature.cycles.toggle');
  });

  it('focus commands and the focus route follow the focus switch', async () => {
    const onIssue = () => listCommands({ ...ctx, focusedIssueId: 1 } as Ctx).map((c) => c.id);
    const { target } = await mountSettings(Features);
    expect(onIssue()).toContain('issue.focus');
    await toggle(target, 'Focus mode');
    expect(isOn('focus')).toBe(false);
    expect(onIssue()).not.toContain('issue.focus');
    expect(ids()).toContain('feature.focus.toggle');
  });

  it('repeat commands follow the recurring switch', async () => {
    const onIssue = () => listCommands({ ...ctx, focusedIssueId: 1 } as Ctx).map((c) => c.id);
    const { target } = await mountSettings(Features);
    expect(onIssue()).toContain('issue.repeat');
    await toggle(target, 'Recurring issues');
    expect(isOn('recurring')).toBe(false);
    expect(onIssue()).not.toContain('issue.repeat');
    expect(onIssue()).not.toContain('issue.repeat.skip');
  });

  it('dashboard and charts keep their existing keys', async () => {
    const { target, api } = await mountSettings(Features);
    await toggle(target, 'Dashboard');
    await toggle(target, 'Progress charts');
    expect(await api.settings.get('dashboard')).toBe(false);
    expect(await api.settings.get('insights_charts')).toBe(false);
  });

  it('has a command per switch', () => {
    for (const f of FEATURES) expect(ids()).toContain(`feature.${f.name}.toggle`);
  });
});

describe('Shutdown ritual switch', () => {
  it('is off until switched on, and its command follows the switch', async () => {
    const { target } = await mountSettings(Features);
    expect(field(target, 'Shutdown ritual').getAttribute('aria-checked')).toBe('false');
    expect(ids()).not.toContain('shutdown.open');
    await toggle(target, 'Shutdown ritual');
    expect(isOn('shutdown')).toBe(true);
    expect(ids()).toContain('shutdown.open');
  });
});

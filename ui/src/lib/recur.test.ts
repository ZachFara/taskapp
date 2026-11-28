import { describe, expect, it } from 'vitest';
import { canonical, nextDue, parseRule, resetChecklist, ruleWords } from './recur';

const next = (text: string, base: string, today = '2020-01-01') => nextDue(parseRule(text)!, base, today);

describe('repeat rules (same cases as crates/core/tests/recur_rule.rs)', () => {
  it('text forms become one canonical rule', () => {
    for (const [said, canon] of [
      ['daily', 'every 1 day from due'], ['Every 3 days', 'every 3 day from due'],
      ['weekdays', 'every 1 week on mon,tue,wed,thu,fri from due'], ['every monday', 'every 1 week on mon from due'],
      ['every thu and mon', 'every 1 week on mon,thu from due'], ['every 2 weeks on tuesday', 'every 2 week on tue from due'],
      ['weekly', 'every 1 week from due'], ['monthly on the 15th', 'every 1 month on 15 from due'], ['every month', 'every 1 month from due'],
      ['every 2 weeks from completion', 'every 2 week from done'],
    ]) expect(canonical(said), said).toBe(canon);
  });
  it('refuses nonsense', () => {
    for (const bad of ['', 'sometimes', 'every 0 days', 'every 2 days on mon', 'every week on 15', 'every month on 32', 'every 1000 days']) expect(parseRule(bad), bad).toBeNull();
  });
  it('plain words', () => {
    expect(ruleWords(parseRule('weekdays')!)).toBe('Every weekday');
    expect(ruleWords(parseRule('every 2 weeks on mon,thu from completion')!)).toBe('Every 2 weeks on Mon and Thu, from completion');
    expect(ruleWords(parseRule('monthly on 31')!)).toBe('Every month on day 31');
  });
  it('steps days, weeks, chosen days and months', () => {
    expect(next('every 3 days', '2026-02-27')).toBe('2026-03-02');
    expect(next('weekly', '2026-12-30')).toBe('2027-01-06');
    expect(next('weekdays', '2026-02-06')).toBe('2026-02-09');
    expect(next('every monday,thursday', '2026-02-05')).toBe('2026-02-09');
    expect(next('every 2 weeks on mon,thu', '2026-02-05')).toBe('2026-02-16');
    expect(next('monthly', '2024-01-31')).toBe('2024-02-29');
    expect(next('monthly on 31', '2026-03-31')).toBe('2026-04-30');
    expect(next('monthly on 15', '2026-12-15')).toBe('2027-01-15');
  });
  it('from due never lands on or before today; from completion counts from today', () => {
    expect(next('every monday', '2026-02-02', '2026-02-25')).toBe('2026-03-02');
    expect(next('every 2 weeks from completion', '2025-06-01', '2026-02-25')).toBe('2026-03-11');
  });
  it('unticks task boxes outside code', () => {
    expect(resetChecklist('intro [x]\n- [x] a\n  * [X] b\n- [ ] c\n```\n- [x] code\n```')).toBe('intro [x]\n- [ ] a\n  * [ ] b\n- [ ] c\n```\n- [x] code\n```');
  });
});

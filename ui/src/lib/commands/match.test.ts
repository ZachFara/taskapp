import { describe, expect, it } from 'vitest';
import { rank } from './match';

const items = [
  { id: 'a', label: 'Set status' },
  { id: 'b', label: 'Create issue', keywords: ['new', 'add'] },
  { id: 'c', label: 'Go to settings' },
];
const ids = (q: string, recent: string[] = []) => rank(q, items, recent).map((x) => x.id);

describe('match', () => {
  it('keeps only subsequence matches', () => expect(ids('zzz')).toEqual([]));
  it('matches on keywords', () => expect(ids('add')).toEqual(['b']));
  it('ranks a tight word-start match above a scattered one', () => expect(ids('set')[0]).toBe('a'));
  it('boosts recent items on equal matches and orders empty query by recency', () => {
    expect(ids('se', ['c'])[0]).toBe('c');
    expect(ids('', ['c', 'b'])).toEqual(['c', 'b', 'a']);
  });
});

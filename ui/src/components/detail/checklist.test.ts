import { describe, expect, it } from 'vitest';
import { checklistCount, checklistLines, toggleLine } from './checklist';

describe('checklist lines', () => {
  it('finds task lines in document order with their line index and state', () => {
    const src = 'intro\n- [ ] one\n- [x] two\n- [X] three\nplain - [ ] not a task';
    expect(checklistLines(src)).toEqual([
      { line: 1, checked: false },
      { line: 2, checked: true },
      { line: 3, checked: true },
    ]);
  });

  it('counts nested task items and any bullet marker', () => {
    const src = '* [ ] parent\n  - [x] child\n    + [ ] grandchild';
    expect(checklistCount(src)).toEqual({ done: 1, total: 3 });
  });

  it('ignores task lines inside fenced code blocks', () => {
    const src = '```md\n- [ ] in code\n```\n- [x] real\n~~~\n- [ ] also code\n~~~';
    expect(checklistCount(src)).toEqual({ done: 1, total: 1 });
  });

  it('reports zero for a description with no checklist', () => {
    expect(checklistCount('just text\n- a bullet')).toEqual({ done: 0, total: 0 });
  });
});

describe('toggling a checklist line', () => {
  it('ticks only the line asked for, even when another line has the same text', () => {
    const src = '- [ ] milk\n- [ ] milk';
    expect(toggleLine(src, 1)).toBe('- [ ] milk\n- [x] milk');
  });

  it('unticks a checked line and keeps the rest of the text', () => {
    expect(toggleLine('a\n- [X] done *now*\nb', 1)).toBe('a\n- [ ] done *now*\nb');
  });
});

// Markdown task lines (`- [ ] item`) in a description. The description stays the source of truth: a tick rewrites its line.
const TASK = /^\s*[-*+]\s+\[([ xX])\](?: |\r?$)/;
const FENCE = /^\s*(```|~~~)/;

/** Task lines outside fenced code, in document order. `line` indexes the description split on newlines. */
export function checklistLines(src: string): { line: number; checked: boolean }[] {
  const out: { line: number; checked: boolean }[] = [];
  let fenced = false;
  src.split('\n').forEach((text, line) => {
    if (FENCE.test(text)) fenced = !fenced;
    else if (!fenced && TASK.test(text)) out.push({ line, checked: TASK.exec(text)![1] !== ' ' });
  });
  return out;
}

export const checklistCount = (src: string) => {
  const items = checklistLines(src);
  return { done: items.filter((i) => i.checked).length, total: items.length };
};

/** The description with the box on one line flipped; every other line is untouched. */
export function toggleLine(src: string, line: number): string {
  const lines = src.split('\n');
  lines[line] = lines[line].replace(/\[([ xX])\]/, (_, c: string) => (c === ' ' ? '[x]' : '[ ]'));
  return lines.join('\n');
}

type Item = { id: string; label: string; keywords?: string[] };

/** Subsequence score of `q` in `text` (higher is better), or null when it is not a subsequence. */
function score(q: string, text: string) {
  let at = -1;
  let s = 0;
  for (const ch of q) {
    const i = text.indexOf(ch, at + 1);
    if (i < 0) return null;
    s += 1 + (i === at + 1 ? 3 : 0) + (i === 0 || text[i - 1] === ' ' ? 4 : 0);
    at = i;
  }
  return s;
}

/** Items matching `q` best first (label and keywords); recent ids get a boost. An empty query lists recent items first. */
export function rank<T extends Item>(q: string, items: T[], recent: string[] = []) {
  const query = q.toLowerCase().replace(/\s+/g, '');
  const boost = (id: string) => (recent.includes(id) ? 10 - recent.indexOf(id) * 0.1 : 0);
  return items
    .flatMap((it) => {
      const best = [it.label, ...(it.keywords ?? [])].map((t) => score(query, t.toLowerCase())).filter((n) => n !== null);
      return best.length ? [{ it, s: Math.max(...best) + boost(it.id) }] : [];
    })
    .sort((a, b) => b.s - a.s)
    .map((x) => x.it);
}

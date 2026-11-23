// The one place a priority number becomes a name, a glyph level or a sort position.
// Backend: 0 or null none, 1 urgent, 2 high, 3 medium, 4 low. The glyph counts the other way (4 rings = urgent).
export const PRIORITIES = ['No priority', 'Urgent', 'High', 'Medium', 'Low'];
export const PRIORITY_OPTIONS = PRIORITIES.map((name, value) => ({ value, name }));
const valid = (p: number | null | undefined): p is number => p != null && p >= 1 && p <= 4;
export const priorityName = (p: number | null | undefined) => PRIORITIES[valid(p) ? p : 0];
export const priorityGlyph = (p: number | null | undefined) => (valid(p) ? 5 - p : 0);
/** Urgent first, none last. */
export const priorityOrder = (p: number | null | undefined) => (valid(p) ? p : 9);

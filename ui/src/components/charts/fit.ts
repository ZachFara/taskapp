/** Reports the element's width now and whenever it changes. Quiet where there is no ResizeObserver (jsdom). */
export function fit(node: HTMLElement, set: (width: number) => void) {
  set(node.clientWidth);
  if (typeof ResizeObserver === 'undefined') return;
  const ro = new ResizeObserver(() => set(node.clientWidth));
  ro.observe(node);
  return { destroy: () => ro.disconnect() };
}

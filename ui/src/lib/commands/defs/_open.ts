import type { Ctx } from '../registry';

/**
 * Goes to the page that owns a form and then fires `event` on window for it to open the form.
 * The event waits one task after a navigation so the page has mounted and is listening.
 */
export function openForm(ctx: Ctx, route: string, event: string) {
  const fire = () => dispatchEvent(new CustomEvent(event));
  if (ctx.route === route) return fire();
  ctx.navigate(route);
  setTimeout(fire, 0);
}

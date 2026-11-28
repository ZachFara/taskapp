import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, fixtures } from './api.mock';
import { copyLink, linkTo, openCairnLink, routeOf } from './deepLink';

afterEach(() => vi.restoreAllMocks());

describe('taskapp:// links', () => {
  it('builds one link per item type', () => {
    expect(linkTo('issue', 'ABC-1')).toBe('taskapp://issue/ABC-1');
    expect(linkTo('project', 'web')).toBe('taskapp://project/web');
    expect(linkTo('cycle', 3)).toBe('taskapp://cycle/3');
  });

  it('copies the link to the clipboard', async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    await copyLink('decision', 4);
    expect(writeText).toHaveBeenCalledWith('taskapp://decision/4');
    vi.unstubAllGlobals();
  });

  it('maps every type to the route that shows it', async () => {
    const api = createMockApi(fixtures);
    const slug = (await api.projects.list())[0].slug;
    const ms = fixtures.milestones?.[0];
    const want: [string, string | null][] = [
      ['taskapp://issue/ABC-1', '/issue/ABC-1'],
      [`taskapp://project/${slug}`, `/project/${slug}`],
      ['taskapp://view/2', '/view/2'],
      ['taskapp://cycle/3', '/cycles#3'],
      ['taskapp://event/9', '/event/9'],
      ['taskapp://decision/4', '/decisions#4'],
      ['taskapp://milestone/99999', null],
      ['taskapp://settings/general', null],
      ['https://example.com', null],
      ['taskapp://view/2/../3', null],
    ];
    for (const [url, route] of want) expect(await routeOf(url, api), url).toBe(route);
    if (ms) expect(await routeOf(`taskapp://milestone/${ms.id}`, api)).toBe(`/project/${(await api.projects.list()).find((p) => p.id === ms.project_id)!.slug}`);
  });

  it('goes to the route, and says false for a link that names nothing', async () => {
    const api = createMockApi(fixtures);
    expect(await openCairnLink('taskapp://event/9', api)).toBe(true);
    expect(location.pathname).toBe('/event/9');
    expect(await openCairnLink('taskapp://nope/1', api)).toBe(false);
  });
});

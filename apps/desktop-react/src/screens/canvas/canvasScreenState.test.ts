import { describe, expect, it } from 'vitest';

import {
  createCanvasDetailTarget,
  openCanvasObjectDetail,
  parseCanvasProjectOptions,
  returnToCanvasGlobalView,
  resolveCanvasGlobalViewLoadProjectId,
  type CanvasDetailTarget,
} from './canvasScreenState';

describe('canvasScreenState', () => {
  it('preserves the selected object id type and project context when opening detail', () => {
    const detailTarget = createCanvasDetailTarget('asset-9', 'asset', 'project-beta');
    const nextView = openCanvasObjectDetail(detailTarget);

    expect(detailTarget).toEqual<CanvasDetailTarget>({
      objectId: 'asset-9',
      objectType: 'asset',
      projectId: 'project-beta',
    });
    expect(nextView).toEqual({
      kind: 'object-detail',
      detailTarget,
    });
  });

  it('uses the selected project id as the next global-view load input', () => {
    expect(resolveCanvasGlobalViewLoadProjectId(null)).toBeNull();
    expect(resolveCanvasGlobalViewLoadProjectId('project-gamma')).toBe('project-gamma');
  });

  it('keeps project navigation data structured at the screen boundary', () => {
    expect(
      parseCanvasProjectOptions([
        { id: 'project-alpha', name: 'Project Alpha' },
        { id: 'project-beta', name: 'Project Beta' },
      ]),
    ).toEqual([
      { id: 'project-alpha', name: 'Project Alpha' },
      { id: 'project-beta', name: 'Project Beta' },
    ]);
  });

  it('returns to the global view without discarding the transition model', () => {
    expect(returnToCanvasGlobalView()).toEqual({ kind: 'global-view' });
  });
});

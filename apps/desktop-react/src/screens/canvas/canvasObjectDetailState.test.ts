import { describe, expect, it } from 'vitest';

import { createCanvasDetailTarget } from './canvasScreenState';
import type { CanvasDetailViewDto } from './types';
import {
  createCanvasObjectDetailLoadInput,
  getCanvasDetailContextNotice,
  getCanvasDetailSummary,
  getCanvasDetailTemplate,
  shouldIncludeProjectInBreadcrumb,
} from './canvasObjectDetailState';

function createDetailView(overrides: Partial<CanvasDetailViewDto> = {}): CanvasDetailViewDto {
  return {
    focusNodeId: 'focus-1',
    focusNodeType: 'project',
    graph: { nodes: [], edges: [] },
    inspectorsByNodeId: {},
    ...overrides,
  };
}

describe('canvasObjectDetailState', () => {
  it('selects the typed detail template for every supported object kind', () => {
    expect(getCanvasDetailTemplate('project')).toBe('project');
    expect(getCanvasDetailTemplate('work_item')).toBe('work_item');
    expect(getCanvasDetailTemplate('asset')).toBe('asset');
    expect(getCanvasDetailTemplate('source')).toBe('source');
    expect(getCanvasDetailTemplate('chunk')).toBe('chunk');
  });

  it('shows a source context notice only when contextual project data exists', () => {
    expect(
      getCanvasDetailContextNotice(
        createDetailView({
          focusNodeId: 'source-1',
          focusNodeType: 'source',
          graph: {
            nodes: [{ id: 'source-1', nodeType: 'source' }],
            edges: [],
          },
          inspectorsByNodeId: {
            'source-1': {
              nodeId: 'source-1',
              nodeType: 'source',
              fields: { title: 'Primary Dataset' },
            },
          },
        }),
      ),
    ).toBeNull();

    expect(
      getCanvasDetailContextNotice(
        createDetailView({
          focusNodeId: 'source-1',
          focusNodeType: 'source',
          graph: {
            nodes: [
              { id: 'project-1', nodeType: 'project' },
              { id: 'source-1', nodeType: 'source' },
            ],
            edges: [],
          },
          inspectorsByNodeId: {
            'project-1': {
              nodeId: 'project-1',
              nodeType: 'project',
              fields: { name: 'Project Delta' },
            },
            'source-1': {
              nodeId: 'source-1',
              nodeType: 'source',
              fields: { title: 'Primary Dataset' },
            },
          },
        }),
      ),
    ).toEqual({
      label: 'Viewed in project context',
      value: 'Project Delta',
      icon: 'visibility',
    });
  });

  it('keeps chunk context source-owned without inventing a parent project field', () => {
    expect(
      getCanvasDetailContextNotice(
        createDetailView({
          focusNodeId: 'chunk-1',
          focusNodeType: 'chunk',
          graph: {
            nodes: [
              { id: 'project-1', nodeType: 'project' },
              { id: 'source-1', nodeType: 'source' },
              { id: 'chunk-1', nodeType: 'chunk' },
            ],
            edges: [{ from: 'source-1', to: 'chunk-1', edgeType: 'source_has_chunk' }],
          },
          inspectorsByNodeId: {
            'project-1': {
              nodeId: 'project-1',
              nodeType: 'project',
              fields: { name: 'Project Delta' },
            },
            'source-1': {
              nodeId: 'source-1',
              nodeType: 'source',
              fields: { title: 'Structural Analysis Dataset' },
            },
            'chunk-1': {
              nodeId: 'chunk-1',
              nodeType: 'chunk',
              fields: {
                title: 'Stress Variance #881',
                parentSource: 'source-1',
              },
            },
          },
        }),
      ),
    ).toEqual({
      label: 'From source',
      value: 'Structural Analysis Dataset',
      icon: 'link',
    });
  });

  it('builds the real detail load input from the screen target without rewriting context', () => {
    expect(
      createCanvasObjectDetailLoadInput(
        createCanvasDetailTarget('chunk-1', 'chunk', 'project-1'),
      ),
    ).toEqual({
      objectId: 'chunk-1',
      objectType: 'chunk',
      projectId: 'project-1',
    });

    expect(
      createCanvasObjectDetailLoadInput(
        createCanvasDetailTarget('source-1', 'source', null),
      ),
    ).toEqual({
      objectId: 'source-1',
      objectType: 'source',
      projectId: null,
    });
  });

  it('keeps source and chunk project context out of the structural breadcrumb', () => {
    expect(shouldIncludeProjectInBreadcrumb('project')).toBe(false);
    expect(shouldIncludeProjectInBreadcrumb('work_item')).toBe(true);
    expect(shouldIncludeProjectInBreadcrumb('asset')).toBe(true);
    expect(shouldIncludeProjectInBreadcrumb('source')).toBe(false);
    expect(shouldIncludeProjectInBreadcrumb('chunk')).toBe(false);
  });

  it('uses only neutral summary fallback text when the dto has no summary', () => {
    expect(
      getCanvasDetailSummary({
        nodeId: 'source-1',
        nodeType: 'source',
        fields: { title: 'Primary Dataset' },
      }),
    ).toBe('Bridge-owned detail metadata is available for this object.');

    expect(
      getCanvasDetailSummary({
        nodeId: 'asset-1',
        nodeType: 'asset',
        fields: { summary: 'Asset summary' },
      }),
    ).toBe('Asset summary');
  });
});

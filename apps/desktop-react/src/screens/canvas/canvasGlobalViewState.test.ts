import { describe, expect, it } from 'vitest';

import {
  createCanvasGlobalViewState,
  selectCanvasGlobalNode,
  type CanvasGlobalViewDto,
} from './canvasGlobalViewState';

const projectNode = { id: 'project-delta', nodeType: 'project' };
const workItemNode = { id: 'wi-1', nodeType: 'work_item' };
const assetNode = { id: 'asset-1', nodeType: 'asset' };

function buildProjection(): CanvasGlobalViewDto {
  return {
    currentProjectId: 'project-delta',
    graph: {
      nodes: [projectNode, workItemNode, assetNode],
      edges: [
        { from: 'project-delta', to: 'wi-1', edgeType: 'project_has_work_item' },
        { from: 'project-delta', to: 'asset-1', edgeType: 'project_has_asset' },
      ],
    },
    inspectorsByNodeId: {
      'project-delta': {
        nodeId: 'project-delta',
        nodeType: 'project',
        fields: {
          name: 'Project Delta',
          summary: 'Project summary',
        },
      },
      'wi-1': {
        nodeId: 'wi-1',
        nodeType: 'work_item',
        fields: {
          projectId: 'project-delta',
          title: 'Structural Audit',
          summary: 'Work item summary',
        },
      },
      'asset-1': {
        nodeId: 'asset-1',
        nodeType: 'asset',
        fields: {
          projectId: 'project-delta',
          title: 'Topology Snapshot',
          summary: 'Asset summary',
        },
      },
    },
  };
}

describe('createCanvasGlobalViewState', () => {
  it('uses the current project as the default selected node when present', () => {
    const state = createCanvasGlobalViewState(buildProjection());

    expect(state.currentProjectId).toBe('project-delta');
    expect(state.selectedNodeId).toBe('project-delta');
    expect(state.selectedInspector?.nodeType).toBe('project');
    expect(state.selectedInspector?.fields.name).toBe('Project Delta');
  });

  it('switches the local inspector payload when project work item or asset is selected', () => {
    const initialState = createCanvasGlobalViewState(buildProjection());
    const workItemState = selectCanvasGlobalNode(initialState, 'wi-1');
    const assetState = selectCanvasGlobalNode(workItemState, 'asset-1');

    expect(workItemState.selectedNodeId).toBe('wi-1');
    expect(workItemState.selectedInspector?.nodeType).toBe('work_item');
    expect(workItemState.selectedInspector?.fields.title).toBe('Structural Audit');

    expect(assetState.selectedNodeId).toBe('asset-1');
    expect(assetState.selectedInspector?.nodeType).toBe('asset');
    expect(assetState.selectedInspector?.fields.title).toBe('Topology Snapshot');
  });

  it('returns an empty selection state when the bridge returns no current project or nodes', () => {
    const state = createCanvasGlobalViewState({
      currentProjectId: null,
      graph: { nodes: [], edges: [] },
      inspectorsByNodeId: {},
    });

    expect(state.currentProjectId).toBeNull();
    expect(state.selectedNodeId).toBeNull();
    expect(state.selectedInspector).toBeNull();
    expect(state.graph.nodes).toHaveLength(0);
  });

  it('does not invent a default selection when the bridge does not provide one', () => {
    const projection = buildProjection();
    const state = createCanvasGlobalViewState({
      ...projection,
      currentProjectId: null,
    });

    expect(state.currentProjectId).toBeNull();
    expect(state.selectedNodeId).toBeNull();
    expect(state.selectedInspector).toBeNull();
  });
});

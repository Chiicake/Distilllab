import type { CanvasGlobalViewDto, CanvasGlobalViewState } from './types';

function resolveDefaultSelectedNodeId(view: CanvasGlobalViewDto): string | null {
  if (view.currentProjectId && view.inspectorsByNodeId[view.currentProjectId]) {
    return view.currentProjectId;
  }

  return null;
}

export function createCanvasGlobalViewState(view: CanvasGlobalViewDto): CanvasGlobalViewState {
  const selectedNodeId = resolveDefaultSelectedNodeId(view);

  return {
    currentProjectId: view.currentProjectId,
    graph: view.graph,
    inspectorsByNodeId: view.inspectorsByNodeId,
    selectedNodeId,
    selectedInspector: selectedNodeId ? view.inspectorsByNodeId[selectedNodeId] ?? null : null,
  };
}

export function selectCanvasGlobalNode(state: CanvasGlobalViewState, nodeId: string): CanvasGlobalViewState {
  const selectedInspector = state.inspectorsByNodeId[nodeId];

  if (!selectedInspector) {
    return state;
  }

  return {
    ...state,
    selectedNodeId: nodeId,
    selectedInspector,
  };
}

export type { CanvasGlobalViewDto } from './types';

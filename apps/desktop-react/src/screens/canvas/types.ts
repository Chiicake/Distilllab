export type CanvasNodeType = 'project' | 'work_item' | 'asset' | 'source' | 'chunk';

export type CanvasGraphNode = {
  id: string;
  nodeType: CanvasNodeType;
};

export type CanvasGraphEdge = {
  from: string;
  to: string;
  edgeType: string;
};

export type CanvasGraphDto = {
  nodes: CanvasGraphNode[];
  edges: CanvasGraphEdge[];
};

export type CanvasInspectorDto = {
  nodeId: string;
  nodeType: CanvasNodeType;
  fields: Record<string, string>;
};

export type CanvasGlobalViewDto = {
  currentProjectId: string | null;
  graph: CanvasGraphDto;
  inspectorsByNodeId: Record<string, CanvasInspectorDto>;
};

export type CanvasDetailViewDto = {
  focusNodeId: string;
  focusNodeType: CanvasNodeType;
  graph: CanvasGraphDto;
  inspectorsByNodeId: Record<string, CanvasInspectorDto>;
};

export type CanvasGlobalViewState = {
  currentProjectId: string | null;
  graph: CanvasGraphDto;
  inspectorsByNodeId: Record<string, CanvasInspectorDto>;
  selectedNodeId: string | null;
  selectedInspector: CanvasInspectorDto | null;
};

import type { CanvasDetailTarget } from './canvasScreenState';
import type { CanvasDetailViewDto, CanvasInspectorDto, CanvasNodeType } from './types';

export type CanvasDetailTemplate = CanvasNodeType;

export type CanvasObjectDetailLoadInput = {
  objectId: string;
  objectType: CanvasNodeType;
  projectId: string | null;
};

export type CanvasDetailContextNotice = {
  label: string;
  value: string;
  icon: string;
};

export function createCanvasObjectDetailLoadInput(detailTarget: CanvasDetailTarget): CanvasObjectDetailLoadInput {
  return {
    objectId: detailTarget.objectId,
    objectType: detailTarget.objectType,
    projectId: detailTarget.projectId,
  };
}

export function getCanvasDetailTemplate(nodeType: CanvasNodeType): CanvasDetailTemplate {
  return nodeType;
}

export function shouldIncludeProjectInBreadcrumb(nodeType: CanvasNodeType): boolean {
  return nodeType === 'work_item' || nodeType === 'asset';
}

export function getCanvasDetailSummary(inspector: CanvasInspectorDto | null | undefined): string {
  return inspector?.fields.summary ?? 'Bridge-owned detail metadata is available for this object.';
}

export function getCanvasDetailContextNotice(detailView: CanvasDetailViewDto): CanvasDetailContextNotice | null {
  const focusInspector = detailView.inspectorsByNodeId[detailView.focusNodeId];
  if (!focusInspector) {
    return null;
  }

  if (detailView.focusNodeType === 'chunk') {
    const sourceId = focusInspector.fields.parentSource;
    if (!sourceId) {
      return null;
    }

    const sourceInspector = detailView.inspectorsByNodeId[sourceId];
    const sourceTitle = getInspectorTitle(sourceInspector);
    if (!sourceTitle) {
      return null;
    }

    return {
      label: 'From source',
      value: sourceTitle,
      icon: 'link',
    };
  }

  if (detailView.focusNodeType === 'source') {
    const projectInspector = getContextProjectInspector(detailView);
    const projectTitle = getInspectorTitle(projectInspector);
    if (!projectTitle) {
      return null;
    }

    return {
      label: 'Viewed in project context',
      value: projectTitle,
      icon: 'visibility',
    };
  }

  return null;
}

function getContextProjectInspector(detailView: CanvasDetailViewDto): CanvasInspectorDto | null {
  const projectNode = detailView.graph.nodes.find((node) => node.nodeType === 'project');
  if (!projectNode) {
    return null;
  }

  return detailView.inspectorsByNodeId[projectNode.id] ?? null;
}

function getInspectorTitle(inspector: CanvasInspectorDto | null | undefined): string | null {
  if (!inspector) {
    return null;
  }

  return inspector.fields.name ?? inspector.fields.title ?? inspector.nodeId;
}

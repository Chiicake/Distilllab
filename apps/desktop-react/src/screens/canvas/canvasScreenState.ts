import type { CanvasNodeType } from './types';

export type CanvasDetailTarget = {
  objectId: string;
  objectType: CanvasNodeType;
  projectId: string | null;
};

export type CanvasProjectOption = {
  id: string;
  name: string;
};

export type CanvasScreenView =
  | { kind: 'global-view' }
  | { kind: 'object-detail'; detailTarget: CanvasDetailTarget };

export function createCanvasDetailTarget(
  objectId: string,
  objectType: CanvasNodeType,
  projectId: string | null,
): CanvasDetailTarget {
  return { objectId, objectType, projectId };
}

export function openCanvasObjectDetail(detailTarget: CanvasDetailTarget): CanvasScreenView {
  return {
    kind: 'object-detail',
    detailTarget,
  };
}

export function returnToCanvasGlobalView(): CanvasScreenView {
  return { kind: 'global-view' };
}

export function parseCanvasProjectOptions(projects: CanvasProjectOption[]): CanvasProjectOption[] {
  return projects;
}

export function resolveCanvasGlobalViewLoadProjectId(projectId: string | null): string | null {
  return projectId;
}

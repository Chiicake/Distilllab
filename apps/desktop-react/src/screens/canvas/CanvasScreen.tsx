import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

import CanvasGlobalView from './CanvasGlobalView';
import CanvasObjectDetail from './CanvasObjectDetail';
import {
  createCanvasDetailTarget,
  openCanvasObjectDetail,
  parseCanvasProjectOptions,
  resolveCanvasGlobalViewLoadProjectId,
  returnToCanvasGlobalView,
  type CanvasProjectOption,
  type CanvasScreenView,
} from './canvasScreenState';
import type { CanvasGlobalViewDto, CanvasNodeType } from './types';

type CanvasScreenProps = {
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

const emptyCanvasGlobalView: CanvasGlobalViewDto = {
  currentProjectId: null,
  graph: { nodes: [], edges: [] },
  inspectorsByNodeId: {},
};

export default function CanvasScreen({ showLeftSidebar, showRightSidebar }: CanvasScreenProps) {
  const [view, setView] = useState<CanvasScreenView>({ kind: 'global-view' });
  const [globalView, setGlobalView] = useState<CanvasGlobalViewDto>(emptyCanvasGlobalView);
  const [projectOptions, setProjectOptions] = useState<CanvasProjectOption[]>([]);
  const [requestedProjectId, setRequestedProjectId] = useState<string | null>(null);
  const [isLoadingGlobalView, setIsLoadingGlobalView] = useState(true);
  const [globalViewError, setGlobalViewError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadGlobalView = async () => {
      setIsLoadingGlobalView(true);
      setGlobalViewError(null);

      try {
        const projectId = resolveCanvasGlobalViewLoadProjectId(requestedProjectId);
        const [projects, projection] = await Promise.all([
          invoke<CanvasProjectOption[]>('list_canvas_projects'),
          invoke<CanvasGlobalViewDto>('load_canvas_global_view', {
            projectId,
          }),
        ]);

        if (!cancelled) {
          setProjectOptions(parseCanvasProjectOptions(projects));
          setGlobalView(projection);
        }
      } catch (error) {
        if (!cancelled) {
          setProjectOptions([]);
          setGlobalView(emptyCanvasGlobalView);
          setGlobalViewError(error instanceof Error ? error.message : 'Failed to load canvas global view.');
        }
      } finally {
        if (!cancelled) {
          setIsLoadingGlobalView(false);
        }
      }
    };

    void loadGlobalView();

    return () => {
      cancelled = true;
    };
  }, [requestedProjectId]);

  const openObjectDetail = (objectId: string, objectType: CanvasNodeType, projectId: string | null) => {
    setView(openCanvasObjectDetail(createCanvasDetailTarget(objectId, objectType, projectId)));
  };

  const selectProject = (projectId: string) => {
    setRequestedProjectId(projectId);
    setView(returnToCanvasGlobalView());
  };

  const returnToGlobalView = () => {
    setView(returnToCanvasGlobalView());
  };

  switch (view.kind) {
    case 'object-detail':
      return (
        <CanvasObjectDetail
          detailTarget={view.detailTarget}
          onReturnToGlobalView={returnToGlobalView}
          showLeftSidebar={showLeftSidebar}
          showRightSidebar={showRightSidebar}
        />
      );
    case 'global-view':
      return (
        <CanvasGlobalView
          globalView={globalView}
          projectOptions={projectOptions}
          isLoading={isLoadingGlobalView}
          loadError={globalViewError}
          onOpenObjectDetail={openObjectDetail}
          onSelectProject={selectProject}
          showLeftSidebar={showLeftSidebar}
          showRightSidebar={showRightSidebar}
        />
      );
  }
}

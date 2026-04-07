import { useEffect, useState } from 'react';

import { createCanvasGlobalViewState, selectCanvasGlobalNode } from './canvasGlobalViewState';
import type { CanvasGlobalViewDto, CanvasGraphNode, CanvasInspectorDto, CanvasNodeType } from './types';

type CanvasGlobalViewProps = {
  globalView: CanvasGlobalViewDto;
  projectOptions: Array<{ id: string; name: string }>;
  isLoading: boolean;
  loadError: string | null;
  onOpenObjectDetail: (objectId: string, objectType: CanvasNodeType, projectId: string | null) => void;
  onSelectProject: (projectId: string) => void;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

const nodeIconByType: Record<'project' | 'work_item' | 'asset', string> = {
  project: 'folder_open',
  work_item: 'account_tree',
  asset: 'inventory_2',
};

const workItemPositions = [
  'top-[20%] left-[25%] z-10',
  'top-[38%] left-[18%] z-10',
  'bottom-[35%] left-[22%] z-10',
  'bottom-[20%] left-[32%] z-10',
];

const assetPositions = [
  'top-[30%] right-[22%] z-10',
  'bottom-[25%] right-[20%] z-10',
  'bottom-[40%] right-[15%] z-10',
  'top-[15%] right-[10%] z-10',
];

function getInspectorTitle(inspector: CanvasInspectorDto) {
  return inspector.fields.name ?? inspector.fields.title ?? inspector.nodeId;
}

function getInspectorSummary(inspector: CanvasInspectorDto) {
  return inspector.fields.summary ?? 'Bridge-owned metadata is available for this node.';
}

function getObjectTypeLabel(nodeType: CanvasNodeType) {
  if (nodeType === 'work_item') {
    return 'Work Item';
  }

  return `${nodeType.charAt(0).toUpperCase()}${nodeType.slice(1)}`;
}

function getProjectIdForInspector(inspector: CanvasInspectorDto, currentProjectId: string | null) {
  return inspector.fields.projectId ?? (inspector.nodeType === 'project' ? inspector.nodeId : currentProjectId);
}

function getNodePosition(node: CanvasGraphNode, nodeIndex: number) {
  if (node.nodeType === 'project') {
    return 'left-1/2 top-1/2 z-20 -translate-x-1/2 -translate-y-1/2';
  }

  if (node.nodeType === 'work_item') {
    return workItemPositions[nodeIndex % workItemPositions.length];
  }

  return assetPositions[nodeIndex % assetPositions.length];
}

function getNodeAnchor(node: CanvasGraphNode, nodeIndex: number) {
  if (node.nodeType === 'project') {
    return { x: 50, y: 50 };
  }

  const positionIndex = node.nodeType === 'work_item' ? nodeIndex % workItemPositions.length : nodeIndex % assetPositions.length;

  if (node.nodeType === 'work_item') {
    return [
      { x: 25, y: 20 },
      { x: 18, y: 38 },
      { x: 22, y: 65 },
      { x: 32, y: 80 },
    ][positionIndex];
  }

  return [
    { x: 78, y: 30 },
    { x: 80, y: 75 },
    { x: 85, y: 60 },
    { x: 90, y: 15 },
  ][positionIndex];
}

function getNodeShellClassName(nodeType: CanvasNodeType, isSelected: boolean) {
  if (nodeType === 'project') {
    return `w-56 rounded-lg bg-surface-container-high p-6 text-center shadow-[0_0_40px_rgba(186,195,255,0.15)] transition-all ${
      isSelected ? 'ring-1 ring-primary/20' : ''
    }`;
  }

  if (nodeType === 'work_item') {
    return `flex items-center gap-3 rounded-md bg-surface-container-low px-4 py-3 text-left shadow-lg transition-colors ${
      isSelected ? 'ring-1 ring-primary/15 bg-surface-container-high' : 'hover:bg-surface-container'
    }`;
  }

  return `flex items-center gap-3 rounded-xl bg-surface-container-highest px-4 py-3 text-left shadow-lg transition-colors ${
    isSelected ? 'ring-1 ring-primary/15 bg-surface-container-high' : 'hover:bg-surface-container-high'
  }`;
}

function renderInspectorFields(inspector: CanvasInspectorDto) {
  const entries = Object.entries(inspector.fields);

  if (entries.length === 0) {
    return (
      <div className="rounded-lg bg-surface-container-low p-4 text-sm text-on-surface-variant">
        No bridge fields are available for this selection.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {entries.map(([key, value]) => (
        <div key={key} className="rounded-lg bg-surface-container-low p-4">
          <p className="mb-1 text-[10px] font-bold uppercase tracking-widest text-on-surface-variant/60">{key}</p>
          <p className="text-sm leading-relaxed text-on-surface">{value}</p>
        </div>
      ))}
    </div>
  );
}

function GlobalCanvasSidebar({
  currentProjectId,
  projectOptions,
  onSelectProject,
}: {
  currentProjectId: string | null;
  projectOptions: Array<{ id: string; name: string }>;
  onSelectProject: (projectId: string) => void;
}) {
  return (
    <aside className="flex h-full w-64 shrink-0 flex-col gap-y-6 bg-[#191a1a] px-4 py-8">
      <div className="flex items-center gap-3 px-2">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary-container">
          <span className="material-symbols-outlined text-primary" style={{ fontVariationSettings: "'FILL' 1" }}>
            architecture
          </span>
        </div>
        <div>
          <h1 className="font-headline text-lg font-bold tracking-tight text-[#bac3ff]">Atelier</h1>
          <p className="text-[10px] uppercase tracking-widest text-on-surface-variant/60">Technical Studio</p>
        </div>
      </div>

      <nav className="flex-1 space-y-1">
        <div className="px-2 pb-2">
          <p className="mb-3 px-2 text-[10px] uppercase tracking-widest text-on-surface-variant/60">Projects</p>
          <div className="space-y-1">
            {projectOptions.map((project) => {
              const isActive = project.id === currentProjectId;

              return (
                <button
                  key={project.id}
                  type="button"
                  className={`relative flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left ${
                    isActive
                      ? 'bg-[#1f2020] font-bold text-[#bac3ff] shadow-[0_0_15px_rgba(186,195,255,0.1)]'
                      : 'text-[#acabaa] transition-colors hover:bg-[#1f2020]/60'
                  }`}
                  onClick={() => onSelectProject(project.id)}
                >
                  {isActive ? <div className="absolute left-0 top-1/2 h-6 w-1 -translate-y-1/2 rounded-full bg-primary" /> : null}
                  <span className="material-symbols-outlined" style={{ fontVariationSettings: "'FILL' 1" }}>
                    folder_open
                  </span>
                  <span className="truncate text-xs uppercase tracking-widest">{project.name}</span>
                </button>
              );
            })}
            <div className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-[#acabaa]">
              <span className="material-symbols-outlined">account_tree</span>
              <span className="text-xs uppercase tracking-widest">WorkItems</span>
            </div>
            <div className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-[#acabaa]">
              <span className="material-symbols-outlined">inventory_2</span>
              <span className="text-xs uppercase tracking-widest">Assets</span>
            </div>
            <div className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-[#acabaa]">
              <span className="material-symbols-outlined">history</span>
              <span className="text-xs uppercase tracking-widest">History</span>
            </div>
          </div>
        </div>
      </nav>

      <div className="space-y-1 border-t border-outline-variant/10 px-2 pt-6">
        <div className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-[#acabaa]">
          <span className="material-symbols-outlined">help</span>
          <span className="text-xs uppercase tracking-widest">Help</span>
        </div>
        <div className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-[#acabaa]">
          <span className="material-symbols-outlined">logout</span>
          <span className="text-xs uppercase tracking-widest">Logout</span>
        </div>
      </div>
    </aside>
  );
}

export default function CanvasGlobalView({
  globalView,
  projectOptions,
  isLoading,
  loadError,
  onOpenObjectDetail,
  onSelectProject,
  showLeftSidebar,
  showRightSidebar,
}: CanvasGlobalViewProps) {
  const [state, setState] = useState(() => createCanvasGlobalViewState(globalView));

  useEffect(() => {
    setState(createCanvasGlobalViewState(globalView));
  }, [globalView]);

  const projectNodes = state.graph.nodes.filter((node) => node.nodeType === 'project');
  const workItemNodes = state.graph.nodes.filter((node) => node.nodeType === 'work_item');
  const assetNodes = state.graph.nodes.filter((node) => node.nodeType === 'asset');
  const selectedInspector = state.selectedInspector;
  const selectedProjectInspector = (state.currentProjectId ? state.inspectorsByNodeId[state.currentProjectId] : null) ?? null;

  const handleSelectNode = (nodeId: string) => {
    setState((currentState) => selectCanvasGlobalNode(currentState, nodeId));
  };

  const handleOpenSelectedDetail = () => {
    if (!selectedInspector) {
      return;
    }

    onOpenObjectDetail(
      selectedInspector.nodeId,
      selectedInspector.nodeType,
      getProjectIdForInspector(selectedInspector, state.currentProjectId),
    );
  };

  return (
    <div className="flex min-w-0 flex-1 overflow-hidden bg-surface text-on-surface">
      {showLeftSidebar ? (
        <GlobalCanvasSidebar
          currentProjectId={state.currentProjectId}
          projectOptions={projectOptions}
          onSelectProject={onSelectProject}
        />
      ) : null}

      <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <header className="z-30 flex h-14 items-center bg-[#0e0e0e]/60 px-6 backdrop-blur-xl">
          <div className="flex items-center gap-2 font-headline text-sm font-medium tracking-wide">
            <span className="text-on-surface-variant/40">Canvas</span>
            <span className="text-on-surface-variant/40">/</span>
            <h1 className="text-[#bac3ff]">{selectedProjectInspector ? getInspectorTitle(selectedProjectInspector) : 'No Project Selected'}</h1>
          </div>
          <nav className="ml-8 flex items-center gap-8">
            <button type="button" className="border-b-2 border-[#bac3ff] pb-1 font-headline text-sm font-medium tracking-wide text-[#bac3ff]">
              {workItemNodes.length} Work Items
            </button>
            <button type="button" className="font-headline text-sm font-medium tracking-wide text-[#acabaa]/60">
              {assetNodes.length} Assets
            </button>
          </nav>
          <div className="ml-auto flex items-center gap-4">
            <div className="flex items-center rounded-full bg-surface-container px-2 py-1">
              <button type="button" className="p-1 text-[#acabaa]/60 transition-colors hover:text-[#f3faff]">
                <span className="material-symbols-outlined">zoom_in</span>
              </button>
              <button type="button" className="p-1 text-[#acabaa]/60 transition-colors hover:text-[#f3faff]">
                <span className="material-symbols-outlined">fit_screen</span>
              </button>
            </div>
            <button type="button" className="p-2 text-[#acabaa]/60 transition-colors hover:text-[#f3faff]">
              <span className="material-symbols-outlined">settings</span>
            </button>
          </div>
        </header>

        <div className="canvas-grid relative flex-1 overflow-hidden">
          <div className="pointer-events-none absolute left-1/4 top-1/4 h-[600px] w-[600px] rounded-full bg-primary/5 blur-[120px]" />
          <div className="pointer-events-none absolute bottom-1/4 right-1/4 h-[500px] w-[500px] rounded-full bg-tertiary/5 blur-[120px]" />

          {isLoading ? (
            <div className="flex h-full items-center justify-center px-8 text-center">
              <div className="w-full max-w-md rounded-lg bg-surface-container-high p-8">
                <div className="mb-4 text-[10px] font-bold uppercase tracking-[0.2em] text-primary">Loading Canvas</div>
                <p className="text-sm leading-relaxed text-on-surface-variant">
                  Resolving the project-centered graph from the bridge projection.
                </p>
              </div>
            </div>
          ) : loadError ? (
            <div className="flex h-full items-center justify-center px-8 text-center">
              <div className="w-full max-w-md rounded-lg bg-surface-container-high p-8">
                <div className="mb-4 text-[10px] font-bold uppercase tracking-[0.2em] text-error">Canvas Load Failed</div>
                <p className="text-sm leading-relaxed text-on-surface-variant">{loadError}</p>
              </div>
            </div>
          ) : state.graph.nodes.length === 0 ? (
            <div className="flex h-full items-center justify-center px-8 text-center">
              <div className="w-full max-w-lg rounded-lg bg-surface-container-high p-10">
                <div className="mb-4 text-[10px] font-bold uppercase tracking-[0.2em] text-secondary">No Canvas Projects</div>
                <h2 className="font-headline text-2xl font-bold tracking-tight text-on-surface">The global view is empty.</h2>
                <p className="mt-3 text-sm leading-relaxed text-on-surface-variant">
                  The bridge did not return a current project, so there are no project, work item, or asset nodes to display yet.
                </p>
              </div>
            </div>
          ) : (
            <>
              <svg className="pointer-events-none absolute inset-0 h-full w-full opacity-20" xmlns="http://www.w3.org/2000/svg">
                {state.graph.edges.map((edge) => {
                  const fromIndex = state.graph.nodes.findIndex((node) => node.id === edge.from);
                  const toIndex = state.graph.nodes.findIndex((node) => node.id === edge.to);
                  const fromNode = fromIndex >= 0 ? state.graph.nodes[fromIndex] : null;
                  const toNode = toIndex >= 0 ? state.graph.nodes[toIndex] : null;

                  if (!fromNode || !toNode) {
                    return null;
                  }

                  const fromAnchor = getNodeAnchor(fromNode, fromIndex);
                  const toAnchor = getNodeAnchor(toNode, toIndex);

                  return (
                    <line
                      key={`${edge.from}-${edge.to}-${edge.edgeType}`}
                      x1={`${fromAnchor.x}%`}
                      y1={`${fromAnchor.y}%`}
                      x2={`${toAnchor.x}%`}
                      y2={`${toAnchor.y}%`}
                      stroke="#bac3ff"
                      strokeDasharray="4 4"
                      strokeWidth="1"
                    />
                  );
                })}
              </svg>

              {state.graph.nodes.map((node, index) => {
                const inspector = state.inspectorsByNodeId[node.id];
                if (!inspector) {
                  return null;
                }

                if (node.nodeType !== 'project' && node.nodeType !== 'work_item' && node.nodeType !== 'asset') {
                  return null;
                }

                const isSelected = state.selectedNodeId === node.id;

                return (
                  <div key={node.id} className={`absolute ${getNodePosition(node, index)}`}>
                    <button type="button" className={getNodeShellClassName(node.nodeType, isSelected)} onClick={() => handleSelectNode(node.id)}>
                      {node.nodeType === 'project' ? (
                        <>
                          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-primary-container">
                            <span className="material-symbols-outlined text-3xl text-primary" style={{ fontVariationSettings: "'FILL' 1" }}>
                              dataset
                            </span>
                          </div>
                          <h3 className="font-headline text-lg font-bold tracking-tight text-primary">{getInspectorTitle(inspector)}</h3>
                          <p className="mt-1 text-[10px] uppercase tracking-widest text-on-surface-variant">Core Repository</p>
                        </>
                      ) : null}

                      {node.nodeType === 'work_item' ? (
                        <>
                          <div className={`h-2 w-2 rounded-full ${isSelected ? 'bg-primary shadow-[0_0_8px_#bac3ff]' : 'bg-primary-dim'}`} />
                          <div className="min-w-0 flex-1">
                            <span className="block text-[10px] font-medium text-on-surface-variant">{node.id}</span>
                            <span className="block truncate text-xs font-semibold text-on-surface">{getInspectorTitle(inspector)}</span>
                          </div>
                          <span className="rounded bg-primary/10 px-2 py-0.5 text-[9px] font-bold uppercase tracking-tight text-primary">Action</span>
                        </>
                      ) : null}

                      {node.nodeType === 'asset' ? (
                        <>
                          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-surface-container-high">
                            <span className="material-symbols-outlined text-sm text-on-surface-variant">{nodeIconByType.asset}</span>
                          </div>
                          <div className="min-w-0 flex-1">
                            <span className="block text-[10px] font-medium text-on-surface-variant">{node.id}</span>
                            <span className="block truncate text-xs font-semibold text-on-surface">{getInspectorTitle(inspector)}</span>
                          </div>
                          <span className="rounded bg-tertiary/10 px-2 py-0.5 text-[9px] font-bold uppercase tracking-tight text-tertiary">Asset</span>
                        </>
                      ) : null}
                    </button>
                  </div>
                );
              })}
            </>
          )}
        </div>
      </main>

      {showRightSidebar ? (
        <aside className="flex h-full w-80 shrink-0 flex-col bg-surface-bright/60 backdrop-blur-[20px]">
          <div className="p-6">
            <div className="mb-3 flex items-center gap-2 text-primary">
              <span className="material-symbols-outlined text-sm">
                {selectedInspector ? nodeIconByType[selectedInspector.nodeType as 'project' | 'work_item' | 'asset'] ?? 'info' : 'info'}
              </span>
              <span className="text-[10px] font-bold uppercase tracking-widest">
                {selectedInspector ? `${getObjectTypeLabel(selectedInspector.nodeType)} Inspector` : 'Canvas Inspector'}
              </span>
            </div>
            <h2 className="mb-2 font-headline text-xl font-bold tracking-tight text-on-surface">
              {selectedInspector ? getInspectorTitle(selectedInspector) : 'No Selection'}
            </h2>
            <p className="text-sm leading-relaxed text-on-surface-variant">
              {selectedInspector ? getInspectorSummary(selectedInspector) : 'Select a project, work item, or asset node to inspect bridge-owned fields.'}
            </p>
          </div>

          <div className="flex-1 overflow-y-auto px-6 pb-6">
            {selectedInspector ? renderInspectorFields(selectedInspector) : null}
          </div>

          <div className="p-6">
            <button
              type="button"
              className="w-full rounded-lg bg-gradient-to-br from-primary to-primary-container py-4 font-headline text-[10px] font-bold uppercase tracking-[0.15em] text-on-primary shadow-[0_4px_20px_rgba(47,63,146,0.3)] transition-transform hover:scale-[1.02] active:scale-95 disabled:cursor-not-allowed disabled:opacity-40"
              disabled={!selectedInspector}
              onClick={handleOpenSelectedDetail}
            >
              Open Detail
            </button>
          </div>
        </aside>
      ) : null}
    </div>
  );
}

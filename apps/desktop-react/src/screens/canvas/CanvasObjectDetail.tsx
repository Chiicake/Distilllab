import { invoke } from '@tauri-apps/api/core';
import { useEffect, useMemo, useState } from 'react';

import {
  createCanvasObjectDetailLoadInput,
  getCanvasDetailContextNotice,
  getCanvasDetailSummary,
  getCanvasDetailTemplate,
  shouldIncludeProjectInBreadcrumb,
} from './canvasObjectDetailState';
import type { CanvasDetailTarget } from './canvasScreenState';
import type { CanvasDetailViewDto, CanvasGraphNode, CanvasInspectorDto, CanvasNodeType } from './types';

type CanvasObjectDetailProps = {
  detailTarget: CanvasDetailTarget;
  onReturnToGlobalView: () => void;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

const emptyDetailView: CanvasDetailViewDto = {
  focusNodeId: '',
  focusNodeType: 'project',
  graph: { nodes: [], edges: [] },
  inspectorsByNodeId: {},
};

const nodeToneByType: Record<CanvasNodeType, string> = {
  project: 'text-secondary',
  work_item: 'text-primary',
  asset: 'text-on-surface-variant',
  source: 'text-secondary',
  chunk: 'text-primary',
};

const nodeBadgeByType: Record<CanvasNodeType, string> = {
  project: 'bg-secondary-container text-secondary-fixed-dim',
  work_item: 'bg-tertiary-container/20 text-tertiary',
  asset: 'bg-surface-container-highest text-on-surface-variant',
  source: 'bg-primary-container/20 text-primary',
  chunk: 'bg-primary-container/20 text-primary',
};

const nodeIconByType: Record<CanvasNodeType, string> = {
  project: 'architecture',
  work_item: 'account_tree',
  asset: 'inventory_2',
  source: 'source',
  chunk: 'data_thresholding',
};

function formatNodeTypeLabel(nodeType: CanvasNodeType) {
  if (nodeType === 'work_item') {
    return 'Work Item';
  }

  return `${nodeType.charAt(0).toUpperCase()}${nodeType.slice(1)}`;
}

function getInspectorTitle(inspector: CanvasInspectorDto | null | undefined) {
  if (!inspector) {
    return 'Unknown Object';
  }

  return inspector.fields.name ?? inspector.fields.title ?? inspector.nodeId;
}

function getInspectorSummary(inspector: CanvasInspectorDto | null | undefined, nodeType: CanvasNodeType) {
  if (!inspector) {
    return `Unable to resolve ${formatNodeTypeLabel(nodeType).toLowerCase()} detail from the bridge projection.`;
  }

  return getCanvasDetailSummary(inspector);
}

function getFocusInspector(detailView: CanvasDetailViewDto) {
  return detailView.inspectorsByNodeId[detailView.focusNodeId] ?? null;
}

function getNodesByType(detailView: CanvasDetailViewDto, nodeType: CanvasNodeType) {
  return detailView.graph.nodes.filter((node) => node.nodeType === nodeType);
}

function getProjectInspector(detailView: CanvasDetailViewDto) {
  const projectNode = detailView.graph.nodes.find((node) => node.nodeType === 'project');
  return projectNode ? detailView.inspectorsByNodeId[projectNode.id] ?? null : null;
}

function getSourceInspector(detailView: CanvasDetailViewDto) {
  const focusInspector = getFocusInspector(detailView);
  if (detailView.focusNodeType === 'source') {
    return focusInspector;
  }

  const sourceId = focusInspector?.fields.parentSource;
  return sourceId ? detailView.inspectorsByNodeId[sourceId] ?? null : null;
}

function renderFieldCards(fields: Array<{ label: string; value: string | null | undefined; tone?: 'primary' | 'secondary' | 'default' }>) {
  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
      {fields
        .filter((field) => field.value)
        .map((field) => (
          <div
            key={field.label}
            className={`rounded-lg bg-surface-container-low p-6 ${
              field.tone === 'primary'
                ? 'border-l-2 border-primary'
                : field.tone === 'secondary'
                  ? 'border-l-2 border-secondary'
                  : ''
            }`}
          >
            <p className="mb-1 text-[10px] uppercase tracking-[0.2em] text-on-surface-variant">{field.label}</p>
            <p className="font-headline text-lg font-bold text-on-surface">{field.value}</p>
          </div>
        ))}
    </div>
  );
}

function renderRelatedNodeCards(nodes: CanvasGraphNode[], inspectorsByNodeId: Record<string, CanvasInspectorDto>, emptyLabel: string) {
  if (nodes.length === 0) {
    return <p className="text-sm leading-relaxed text-on-surface-variant">{emptyLabel}</p>;
  }

  return (
    <div className="space-y-3">
      {nodes.map((node) => {
        const inspector = inspectorsByNodeId[node.id];
        return (
          <div key={node.id} className="rounded-lg bg-surface-container p-5 transition-colors hover:bg-surface-container-high">
            <div className="mb-2 flex items-start justify-between gap-4">
              <h4 className="font-headline text-lg font-bold text-on-surface">{getInspectorTitle(inspector)}</h4>
              <span className={`rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest ${nodeBadgeByType[node.nodeType]}`}>
                {formatNodeTypeLabel(node.nodeType)}
              </span>
            </div>
            <p className="text-sm leading-relaxed text-on-surface-variant">{getInspectorSummary(inspector, node.nodeType)}</p>
            <p className="mt-3 text-[10px] uppercase tracking-[0.18em] text-on-surface-variant/60">{node.id}</p>
          </div>
        );
      })}
    </div>
  );
}

function renderPrimarySections(detailView: CanvasDetailViewDto) {
  const focusInspector = getFocusInspector(detailView);
  const template = getCanvasDetailTemplate(detailView.focusNodeType);
  const projectInspector = getProjectInspector(detailView);
  const sourceInspector = getSourceInspector(detailView);

  switch (template) {
    case 'project':
      return (
        <>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-primary">01 / Overview</h3>
            {renderFieldCards([
              { label: 'Project Name', value: focusInspector?.fields.name, tone: 'secondary' },
              { label: 'Work Items', value: String(getNodesByType(detailView, 'work_item').length), tone: 'primary' },
              { label: 'Assets', value: String(getNodesByType(detailView, 'asset').length) },
              { label: 'Node Id', value: focusInspector?.nodeId },
            ])}
          </section>
          <section className="space-y-6">
            <div className="flex items-end justify-between gap-4">
              <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-primary">02 / Work Items</h3>
              <span className="text-[10px] uppercase tracking-[0.18em] text-on-surface-variant">Bridge Detail Graph</span>
            </div>
            {renderRelatedNodeCards(getNodesByType(detailView, 'work_item'), detailView.inspectorsByNodeId, 'No work items are present in this project detail projection.')}
          </section>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-primary">03 / Assets</h3>
            {renderRelatedNodeCards(getNodesByType(detailView, 'asset'), detailView.inspectorsByNodeId, 'No assets are present in this project detail projection.')}
          </section>
        </>
      );
    case 'work_item':
      return (
        <>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant/60">Summary & Objectives</h3>
            <div className="rounded-lg bg-surface-container-low p-8">
              <p className="text-lg leading-loose text-on-surface">{getInspectorSummary(focusInspector, 'work_item')}</p>
            </div>
          </section>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant/60">Context: Parent Project</h3>
            {renderRelatedNodeCards(
              getProjectInspector(detailView) ? [{ id: projectInspector!.nodeId, nodeType: 'project' }] : [],
              detailView.inspectorsByNodeId,
              'No parent project context was returned for this work item.',
            )}
          </section>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant/60">Other Work Items In This Project</h3>
            {renderRelatedNodeCards(
              getNodesByType(detailView, 'work_item').filter((node) => node.id !== detailView.focusNodeId),
              detailView.inspectorsByNodeId,
              'No sibling work items were returned by the bridge projection.',
            )}
          </section>
        </>
      );
    case 'asset':
      return (
        <>
          <section className="rounded-xl bg-surface-container-low p-8">
            <h3 className="mb-6 text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant">Asset Overview</h3>
            {renderFieldCards([
              { label: 'Title', value: focusInspector?.fields.title },
              { label: 'Project Id', value: focusInspector?.fields.projectId, tone: 'primary' },
              { label: 'Node Id', value: focusInspector?.nodeId },
              { label: 'Summary', value: focusInspector?.fields.summary, tone: 'secondary' },
            ])}
          </section>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant">Parent Project</h3>
            {renderRelatedNodeCards(
              projectInspector ? [{ id: projectInspector.nodeId, nodeType: 'project' }] : [],
              detailView.inspectorsByNodeId,
              'No parent project was returned for this asset.',
            )}
          </section>
          <section className="space-y-6">
            <h3 className="text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant">Work Items In Project Context</h3>
            {renderRelatedNodeCards(getNodesByType(detailView, 'work_item'), detailView.inspectorsByNodeId, 'No work items were returned alongside this asset.')}
          </section>
        </>
      );
    case 'source':
      return (
        <>
          <section className="space-y-6">
            <div className="flex items-center justify-between gap-4">
              <h3 className="font-headline text-xl font-bold tracking-tight text-on-surface">Source Overview</h3>
              <div className="h-px flex-1 bg-outline-variant/20" />
            </div>
            {renderFieldCards([
              { label: 'Title', value: focusInspector?.fields.title, tone: 'secondary' },
              { label: 'Run Id', value: focusInspector?.fields.runId, tone: 'primary' },
              { label: 'Chunk Count', value: String(getNodesByType(detailView, 'chunk').length) },
              { label: 'Node Id', value: focusInspector?.nodeId },
            ])}
          </section>
          <section className="space-y-6">
            <div className="flex items-center justify-between gap-4">
              <h3 className="font-headline text-xl font-bold tracking-tight text-on-surface">Child Chunks</h3>
              <div className="h-px flex-1 bg-outline-variant/20" />
            </div>
            {renderRelatedNodeCards(getNodesByType(detailView, 'chunk'), detailView.inspectorsByNodeId, 'No chunks were returned for this source.')}
          </section>
        </>
      );
    case 'chunk':
      return (
        <>
          <section className="rounded-lg bg-surface-container-low p-8">
            <div className="mb-6 flex items-center justify-between gap-4">
              <h3 className="text-xs uppercase tracking-[0.2em] text-on-surface-variant">Extracted Fragment Content</h3>
              <span className="text-[10px] font-mono text-tertiary/50">{focusInspector?.nodeId}</span>
            </div>
            <p className="text-sm leading-loose text-on-surface">{getInspectorSummary(focusInspector, 'chunk')}</p>
          </section>
          <section className="space-y-6">
            <h3 className="text-xs uppercase tracking-[0.2em] text-on-surface-variant">Parent Source Context</h3>
            {renderRelatedNodeCards(
              sourceInspector ? [{ id: sourceInspector.nodeId, nodeType: 'source' }] : [],
              detailView.inspectorsByNodeId,
              'No parent source was returned for this chunk.',
            )}
          </section>
        </>
      );
  }
}

function renderInspectorSidebar(detailView: CanvasDetailViewDto) {
  const focusInspector = getFocusInspector(detailView);
  const fields = focusInspector ? Object.entries(focusInspector.fields) : [];
  const relatedNodes = detailView.graph.nodes.filter((node) => node.id !== detailView.focusNodeId);

  return (
    <>
      <div className="p-8">
        <div className="mb-6 flex items-center justify-between">
          <h3 className="font-headline text-xs font-bold uppercase tracking-[0.2em] text-primary">Deep Inspector</h3>
          <span className="material-symbols-outlined text-sm text-on-surface-variant">info</span>
        </div>
        <div className="space-y-4">
          <div className="flex justify-between border-b border-outline-variant/10 py-2">
            <span className="text-xs text-on-surface-variant">Object Id</span>
            <span className="text-xs text-on-surface">{focusInspector?.nodeId ?? detailView.focusNodeId}</span>
          </div>
          <div className="flex justify-between border-b border-outline-variant/10 py-2">
            <span className="text-xs text-on-surface-variant">Type</span>
            <span className="text-xs text-on-surface">{formatNodeTypeLabel(detailView.focusNodeType)}</span>
          </div>
          <div className="flex justify-between border-b border-outline-variant/10 py-2">
            <span className="text-xs text-on-surface-variant">Graph Nodes</span>
            <span className="text-xs text-on-surface">{detailView.graph.nodes.length}</span>
          </div>
          <div className="flex justify-between border-b border-outline-variant/10 py-2">
            <span className="text-xs text-on-surface-variant">Graph Edges</span>
            <span className="text-xs text-on-surface">{detailView.graph.edges.length}</span>
          </div>
        </div>
      </div>
      <div className="px-8 pb-8">
        <h4 className="mb-4 text-[10px] uppercase tracking-[0.2em] text-on-surface-variant">Bridge Fields</h4>
        <div className="space-y-3">
          {fields.length === 0 ? (
            <div className="rounded-lg bg-surface-container-low p-4 text-sm text-on-surface-variant">
              No bridge fields are available for this selection.
            </div>
          ) : (
            fields.map(([key, value]) => (
              <div key={key} className="rounded-lg bg-surface-container-low p-4">
                <p className="mb-1 text-[10px] uppercase tracking-[0.18em] text-on-surface-variant">{key}</p>
                <p className="text-sm leading-relaxed text-on-surface">{value}</p>
              </div>
            ))
          )}
        </div>
      </div>
      <div className="px-8 pb-8">
        <h4 className="mb-4 text-[10px] uppercase tracking-[0.2em] text-on-surface-variant">Related Graph Nodes</h4>
        <div className="space-y-3">
          {relatedNodes.length === 0 ? (
            <div className="rounded-lg bg-surface-container-low p-4 text-sm text-on-surface-variant">
              No related nodes were included in this detail projection.
            </div>
          ) : (
            relatedNodes.map((node) => (
              <div key={node.id} className="flex items-center gap-3 rounded-lg bg-surface-container-low p-3">
                <div className="flex h-8 w-8 items-center justify-center rounded bg-surface-container-highest">
                  <span className="material-symbols-outlined text-sm text-on-surface-variant">{nodeIconByType[node.nodeType]}</span>
                </div>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-xs font-semibold text-on-surface">{getInspectorTitle(detailView.inspectorsByNodeId[node.id])}</p>
                  <p className="text-[10px] uppercase tracking-[0.18em] text-on-surface-variant">{formatNodeTypeLabel(node.nodeType)}</p>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </>
  );
}

function DetailCanvasSidebar() {
  return (
    <aside className="hidden h-full w-64 shrink-0 flex-col gap-y-6 bg-[#191a1a] px-4 py-8 md:flex">
      <div className="flex items-center gap-3 px-2">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary-container">
          <span className="material-symbols-outlined text-sm text-primary" style={{ fontVariationSettings: "'FILL' 1" }}>
            architecture
          </span>
        </div>
        <div>
          <h1 className="font-headline text-lg font-bold tracking-tight text-[#bac3ff]">Atelier</h1>
          <p className="text-[10px] uppercase tracking-widest text-on-surface-variant/60">Technical Studio</p>
        </div>
      </div>

      <nav className="flex-1 space-y-1">
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">folder_open</span>
          <span className="text-xs uppercase tracking-widest">Projects</span>
        </div>
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">account_tree</span>
          <span className="text-xs uppercase tracking-widest">WorkItems</span>
        </div>
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">inventory_2</span>
          <span className="text-xs uppercase tracking-widest">Assets</span>
        </div>
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">history</span>
          <span className="text-xs uppercase tracking-widest">History</span>
        </div>
      </nav>

      <div className="space-y-1 border-t border-outline-variant/10 pt-6">
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">help</span>
          <span className="text-xs uppercase tracking-widest">Help</span>
        </div>
        <div className="flex items-center gap-3 rounded-lg px-3 py-2 text-[#acabaa] transition-colors hover:bg-[#1f2020] hover:text-[#f3faff]">
          <span className="material-symbols-outlined">logout</span>
          <span className="text-xs uppercase tracking-widest">Logout</span>
        </div>
      </div>
    </aside>
  );
}

export default function CanvasObjectDetail({ detailTarget, onReturnToGlobalView, showLeftSidebar, showRightSidebar }: CanvasObjectDetailProps) {
  const [detailView, setDetailView] = useState<CanvasDetailViewDto>(emptyDetailView);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadDetail = async () => {
      setIsLoading(true);
      setLoadError(null);

      try {
        const loadInput = createCanvasObjectDetailLoadInput(detailTarget);
        const projection = await invoke<CanvasDetailViewDto>('load_canvas_object_detail', {
          objectType: loadInput.objectType,
          objectId: loadInput.objectId,
          projectId: loadInput.projectId,
        });
        if (!cancelled) {
          setDetailView(projection);
        }
      } catch (error) {
        if (!cancelled) {
          setDetailView(emptyDetailView);
          setLoadError(error instanceof Error ? error.message : 'Failed to load canvas object detail.');
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    };

    void loadDetail();

    return () => {
      cancelled = true;
    };
  }, [detailTarget]);

  const resolvedFocusType = detailView.focusNodeId ? detailView.focusNodeType : detailTarget.objectType;
  const focusInspector = useMemo(() => getFocusInspector(detailView), [detailView]);
  const detailTitle = focusInspector ? getInspectorTitle(focusInspector) : detailTarget.objectId;
  const detailSummary = focusInspector ? getInspectorSummary(focusInspector, resolvedFocusType) : 'Loading detail from the bridge projection.';
  const contextNotice = detailView.focusNodeId ? getCanvasDetailContextNotice(detailView) : null;
  const includeProjectInBreadcrumb = shouldIncludeProjectInBreadcrumb(resolvedFocusType);

  return (
    <div className="flex min-w-0 flex-1 overflow-hidden bg-surface text-on-surface">
      {showLeftSidebar ? <DetailCanvasSidebar /> : null}

      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <header className="sticky top-0 z-30 flex items-center justify-between bg-[#0e0e0e]/60 px-6 py-3 backdrop-blur-xl">
          <div className="flex items-center gap-4">
            <button
              type="button"
              className="flex items-center gap-2 rounded-full bg-surface-container px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.16em] text-primary transition-colors hover:bg-surface-container-high"
              aria-label="Return to global canvas view"
              onClick={onReturnToGlobalView}
            >
              <span className="material-symbols-outlined text-base">arrow_back</span>
              Return
            </button>
            <div className="flex items-center gap-2 font-headline text-sm font-medium tracking-wide">
              <span className="text-on-surface-variant/60">Canvas</span>
              <span className="text-on-surface-variant/60">/</span>
              {includeProjectInBreadcrumb && detailTarget.projectId ? <span className="text-on-surface-variant/60">{detailTarget.projectId}</span> : null}
              {includeProjectInBreadcrumb && detailTarget.projectId ? <span className="text-on-surface-variant/60">/</span> : null}
              <span className="border-b-2 border-[#bac3ff] pb-1 text-[#bac3ff]">{formatNodeTypeLabel(resolvedFocusType)}</span>
            </div>
            <span className={`rounded px-2 py-0.5 text-[10px] font-bold uppercase tracking-tight ${nodeBadgeByType[resolvedFocusType]}`}>
              {resolvedFocusType.toUpperCase().replace('_', ' ')}
            </span>
          </div>
          <div className="flex items-center gap-6 text-[#acabaa]/60">
            <button type="button" className="transition-colors hover:text-[#f3faff]">
              <span className="material-symbols-outlined">zoom_in</span>
            </button>
            <button type="button" className="transition-colors hover:text-[#f3faff]">
              <span className="material-symbols-outlined">fit_screen</span>
            </button>
            <button type="button" className="transition-colors hover:text-[#f3faff]">
              <span className="material-symbols-outlined">settings</span>
            </button>
          </div>
        </header>

        <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
          <div className="flex-1 overflow-y-auto px-8 py-10 md:px-12">
            {isLoading ? (
              <div className="mx-auto flex max-w-3xl items-center justify-center rounded-lg bg-surface-container-high p-10 text-center">
                <div>
                  <div className="mb-4 text-[10px] font-bold uppercase tracking-[0.2em] text-primary">Loading Detail</div>
                  <p className="text-sm leading-relaxed text-on-surface-variant">Resolving the typed canvas detail projection from `load_canvas_object_detail`.</p>
                </div>
              </div>
            ) : loadError ? (
              <div className="mx-auto flex max-w-3xl items-center justify-center rounded-lg bg-surface-container-high p-10 text-center">
                <div>
                  <div className="mb-4 text-[10px] font-bold uppercase tracking-[0.2em] text-error">Detail Load Failed</div>
                  <p className="text-sm leading-relaxed text-on-surface-variant">{loadError}</p>
                </div>
              </div>
            ) : (
              <div className="mx-auto grid max-w-6xl grid-cols-1 gap-12 lg:grid-cols-12">
                <div className="space-y-12 lg:col-span-8">
                  <section className="space-y-6">
                    <div className="flex items-start gap-8">
                      <div className="flex h-24 w-24 items-center justify-center rounded-lg bg-surface-container-high text-4xl">
                        <span className={`material-symbols-outlined ${nodeToneByType[resolvedFocusType]}`} style={{ fontVariationSettings: "'FILL' 1" }}>
                          {nodeIconByType[resolvedFocusType]}
                        </span>
                      </div>
                      <div className="flex-1 space-y-4">
                        {contextNotice ? (
                          <div className="flex items-center gap-2 text-xs uppercase tracking-[0.2em] text-on-surface-variant">
                            <span className="material-symbols-outlined text-sm text-secondary">{contextNotice.icon}</span>
                            <span>{contextNotice.label}</span>
                            <span className="h-1 w-1 rounded-full bg-outline-variant" />
                            <span className="text-secondary">{contextNotice.value}</span>
                          </div>
                        ) : null}
                        <h1 className="font-headline text-5xl font-extrabold tracking-tight text-on-surface">{detailTitle}</h1>
                        <p className="max-w-3xl text-xl font-light leading-relaxed text-on-surface-variant">{detailSummary}</p>
                      </div>
                    </div>
                  </section>

                  {renderPrimarySections(detailView)}
                </div>

                {showRightSidebar ? (
                  <aside className="sticky top-24 h-fit rounded-xl bg-surface-bright/60 backdrop-blur-[20px] lg:col-span-4">
                    {renderInspectorSidebar(detailView)}
                  </aside>
                ) : null}
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}

import CanvasSidebar from './CanvasSidebar';

type CanvasObjectDetailProps = {
  onReturnToGlobalView: () => void;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

const relatedObjects = [
  {
    icon: 'token',
    iconClassName: 'bg-primary/10 text-primary',
    label: 'Project Delta',
    detail: 'Project',
    actionIcon: 'arrow_outward',
  },
  {
    icon: 'folder_zip',
    iconClassName: 'bg-secondary-container/30 text-secondary',
    label: 'log_files.zip',
    detail: '14.2 MB',
    actionIcon: 'download',
  },
  {
    icon: 'picture_as_pdf',
    iconClassName: 'bg-tertiary-container/10 text-tertiary',
    label: 'summary_report.pdf',
    detail: '2.1 MB',
    actionIcon: 'open_in_new',
  },
];

const recentActivity = [
  {
    title: 'Simulation completed',
    meta: 'System • 2m ago',
    markerClassName: 'bg-primary shadow-[0_0_8px_rgba(186,195,255,0.6)]',
  },
  {
    title: 'Parameter update',
    meta: 'Chat context • 45m ago',
    markerClassName: 'bg-outline-variant',
  },
];

export default function CanvasObjectDetail({ onReturnToGlobalView, showLeftSidebar, showRightSidebar }: CanvasObjectDetailProps) {
  return (
    <div className="flex min-w-0 flex-1 overflow-hidden bg-surface text-on-surface">
      {showLeftSidebar ? <CanvasSidebar activeView="recent-active" /> : null}

      <main className="relative flex min-w-0 flex-1 overflow-hidden bg-surface">
        <div className="absolute right-0 top-0 -z-0 h-[500px] w-[500px] rounded-full bg-primary/5 blur-[120px] pointer-events-none" />

        <div className="flex min-w-0 flex-1 flex-col">
          <header className="z-10 flex h-14 items-center justify-between border-b border-outline-variant/10 bg-surface/50 px-8 backdrop-blur-md">
            <div className="flex items-center gap-3">
              <button
                type="button"
                className="flex items-center gap-2 rounded-full border border-primary/20 bg-surface-container-highest px-3 py-1.5 text-xs font-semibold uppercase tracking-[0.16em] text-primary transition-colors hover:bg-surface-bright"
                aria-label="Return to global canvas view"
                title="Return to global canvas view"
                onClick={onReturnToGlobalView}
              >
                <span className="material-symbols-outlined text-base">arrow_back</span>
                Return
              </button>
              <div className="hidden md:flex items-center gap-2 text-sm text-on-surface-variant">
                <span>Canvas</span>
                <span className="material-symbols-outlined text-xs">chevron_right</span>
                <span className="text-on-surface">Analysis Detail</span>
              </div>
            </div>

            <div className="flex items-center gap-4 text-xs text-on-surface-variant">
              <div className="flex items-center gap-2">
                <span className="h-2 w-2 animate-pulse rounded-full bg-primary" />
                <span>Focus: Analysis</span>
              </div>
              <div className="h-4 w-px bg-outline-variant/30" />
              <span>Object: AN-402-DELTA</span>
            </div>
          </header>

          <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
            <div className="relative flex min-w-0 flex-1 overflow-auto bg-surface p-12">
              <div
                className="pointer-events-none absolute inset-0 opacity-[0.03]"
                style={{
                  backgroundImage: 'radial-gradient(#bac3ff 0.5px, transparent 0.5px)',
                  backgroundSize: '24px 24px',
                }}
              />

              <div className="absolute left-[12%] top-[18%] w-44 rounded-xl border border-outline-variant/10 bg-surface-container-highest/80 px-4 py-2 shadow-lg opacity-60">
                <div className="mb-1 text-[10px] uppercase tracking-[0.16em] text-on-surface-variant">Neighbor</div>
                <div className="text-xs font-semibold text-on-surface">Review</div>
              </div>

              <div className="absolute right-[18%] top-[22%] w-40 rounded-xl border border-outline-variant/10 bg-surface-container-highest/80 px-4 py-2 shadow-lg opacity-50">
                <div className="mb-1 text-[10px] uppercase tracking-[0.16em] text-on-surface-variant">Asset</div>
                <div className="text-xs font-semibold text-on-surface">summary_report.pdf</div>
              </div>

              <div className="absolute bottom-[18%] left-[18%] w-40 rounded-xl border border-outline-variant/10 bg-surface-container-highest/70 px-4 py-2 shadow-lg opacity-45">
                <div className="mb-1 text-[10px] uppercase tracking-[0.16em] text-on-surface-variant">Project</div>
                <div className="text-xs font-semibold text-on-surface">Project Delta</div>
              </div>

              <svg className="pointer-events-none absolute inset-0 h-full w-full opacity-15" xmlns="http://www.w3.org/2000/svg">
                <path d="M 360 250 L 650 360" fill="none" stroke="#bac3ff" strokeDasharray="4 4" strokeWidth="1" />
                <path d="M 980 280 L 760 360" fill="none" stroke="#767575" strokeWidth="1" />
                <path d="M 410 690 L 650 470" fill="none" stroke="#89a1ae" strokeWidth="1" />
              </svg>

              <div className="flex min-h-full w-full items-center justify-center">
                <div className="relative w-full max-w-2xl">
                  <div className="absolute -inset-4 rounded-full bg-primary/5 blur-3xl" />
                  <div className="relative rounded-md border border-primary/20 bg-surface-container p-8 shadow-2xl ring-1 ring-primary/10">
                    <div className="mb-6 flex items-start justify-between gap-4">
                      <div className="flex items-center gap-4">
                        <div className="flex h-12 w-12 items-center justify-center rounded border-l-2 border-primary bg-primary/10">
                          <span className="material-symbols-outlined text-primary">analytics</span>
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <h1 className="font-headline text-3xl font-bold tracking-tight text-on-surface">Analysis</h1>
                            <span className="rounded bg-primary/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-primary">
                              Selected
                            </span>
                          </div>
                          <p className="mt-1 text-sm text-on-surface-variant">WorkItem / AN-402-DELTA</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="h-2 w-2 rounded-full bg-primary animate-pulse" />
                        <span className="text-xs font-semibold uppercase tracking-wider text-primary">In Progress</span>
                      </div>
                    </div>

                    <div className="mb-8">
                      <p className="text-lg leading-relaxed text-on-surface">
                        Analyzing the latest results from the Delta simulation. Correlating telemetry with
                        baseline architectural patterns to identify structural drift in the current sprint.
                      </p>
                    </div>

                    <div className="mb-8 rounded-lg border border-outline-variant/10 bg-surface-container-low p-6">
                      <div className="mb-4 flex items-center justify-between">
                        <div className="text-[10px] font-bold uppercase tracking-[0.18em] text-primary">
                          Local Neighborhood
                        </div>
                        <span className="text-[10px] uppercase tracking-[0.16em] text-on-surface-variant">
                          Recent Active Focus
                        </span>
                      </div>
                      <div className="flex h-24 items-end justify-between gap-2">
                        <div className="h-[40%] w-full rounded-t-sm bg-primary/20" />
                        <div className="h-[65%] w-full rounded-t-sm bg-primary/40" />
                        <div className="h-[50%] w-full rounded-t-sm bg-primary/30" />
                        <div className="h-[90%] w-full rounded-t-sm bg-primary" />
                        <div className="h-[75%] w-full rounded-t-sm bg-primary/50" />
                        <div className="h-[85%] w-full rounded-t-sm bg-primary/60" />
                        <div className="h-[45%] w-full rounded-t-sm bg-primary/30" />
                      </div>
                      <div className="mt-4 flex justify-between text-[10px] uppercase tracking-widest text-on-surface-variant opacity-50">
                        <span>T-Minus 12h</span>
                        <span>Real-time Stream</span>
                        <span>Current Phase</span>
                      </div>
                    </div>

                    <div className="flex gap-4 border-t border-outline-variant/10 pt-6">
                      <div className="flex flex-1 items-center justify-center gap-2 rounded bg-primary-container py-3 text-xs font-bold uppercase tracking-widest text-on-primary-container">
                        <span className="material-symbols-outlined text-sm">chat</span>
                        Open Related Chat
                      </div>
                      <div className="flex flex-1 items-center justify-center gap-2 rounded bg-surface-container-highest py-3 text-xs font-bold uppercase tracking-widest text-on-surface">
                        <span className="material-symbols-outlined text-sm">visibility</span>
                        Full Screen
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {showRightSidebar ? (
            <aside className="z-20 flex h-full w-80 shrink-0 flex-col border-l border-outline-variant/10 bg-[rgba(43,44,44,0.6)] backdrop-blur-[20px]">
              <div className="border-b border-outline-variant/10 p-6">
                <div className="mb-1 flex items-center justify-between">
                  <span className="font-headline text-[10px] font-bold uppercase tracking-[0.2em] text-primary">
                    Inspector
                  </span>
                  <span className="material-symbols-outlined text-on-surface-variant">
                    close
                  </span>
                </div>
                <h2 className="font-headline text-xl font-extrabold text-on-surface">Analysis</h2>
              </div>

              <div className="flex-grow space-y-8 overflow-y-auto p-6">
                <section>
                  <label className="mb-3 block text-[10px] uppercase tracking-widest text-on-surface-variant">
                    Status
                  </label>
                  <div className="flex items-center gap-4 rounded-xl bg-surface-container-high p-4">
                    <div className="relative h-10 w-10">
                      <svg className="h-full w-full -rotate-90 transform" viewBox="0 0 40 40">
                        <circle cx="20" cy="20" r="18" fill="transparent" stroke="currentColor" strokeWidth="3" className="text-outline-variant/20" />
                        <circle
                          cx="20"
                          cy="20"
                          r="18"
                          fill="transparent"
                          stroke="currentColor"
                          strokeWidth="3"
                          strokeDasharray="113"
                          strokeDashoffset="28"
                          className="text-primary"
                        />
                      </svg>
                      <span className="absolute inset-0 flex items-center justify-center text-[10px] font-bold text-primary">
                        75%
                      </span>
                    </div>
                    <div>
                      <div className="text-sm font-semibold text-on-surface">In Progress</div>
                      <div className="text-[11px] text-on-surface-variant">75% complete</div>
                    </div>
                  </div>
                </section>

                <section>
                  <label className="mb-3 block text-[10px] uppercase tracking-widest text-on-surface-variant">
                    Related Objects
                  </label>
                  <div className="space-y-3">
                    {relatedObjects.map((obj) => (
                      <div
                        key={obj.label}
                        className="flex items-center gap-3 rounded-xl border border-outline-variant/10 bg-surface-container-high p-3"
                      >
                        <div className={`flex h-8 w-8 items-center justify-center rounded ${obj.iconClassName}`}>
                          <span className="material-symbols-outlined text-sm">{obj.icon}</span>
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="text-xs font-semibold text-on-surface">{obj.label}</div>
                          <div className="text-[10px] text-on-surface-variant">{obj.detail}</div>
                        </div>
                        <span className="material-symbols-outlined text-sm text-on-surface-variant">
                          {obj.actionIcon}
                        </span>
                      </div>
                    ))}
                  </div>
                </section>

                <section>
                  <label className="mb-3 block text-[10px] uppercase tracking-widest text-on-surface-variant">
                    Recent Activity
                  </label>
                  <div className="space-y-4">
                    {recentActivity.map((item) => (
                      <div key={item.title} className="flex gap-3">
                        <div className="flex flex-col items-center">
                          <div className={`h-2 w-2 rounded-full ${item.markerClassName}`} />
                          <div className="h-full w-px bg-outline-variant/20" />
                        </div>
                        <div className="pb-4">
                          <div className="text-xs font-medium text-on-surface">{item.title}</div>
                          <div className="text-[10px] text-on-surface-variant">{item.meta}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                </section>
              </div>
            </aside>
            ) : null}
          </div>
        </div>
      </main>
    </div>
  );
}

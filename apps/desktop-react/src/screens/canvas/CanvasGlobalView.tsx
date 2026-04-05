import CanvasSidebar from './CanvasSidebar';

type CanvasGlobalViewProps = {
  onOpenObjectDetail: () => void;
  showLeftSidebar: boolean;
};

export default function CanvasGlobalView({ onOpenObjectDetail, showLeftSidebar }: CanvasGlobalViewProps) {
  return (
    <div className="flex min-w-0 flex-1 overflow-hidden bg-surface text-on-surface">
      {showLeftSidebar ? <CanvasSidebar activeView="recent-active" showInventory /> : null}

      <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <header className="z-30 flex h-14 items-center border-b border-outline-variant/10 bg-surface/50 px-8 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <span className="text-on-surface-variant">Scope</span>
            <span className="material-symbols-outlined text-xs text-on-surface-variant">chevron_right</span>
            <h1 className="font-headline font-bold text-on-surface">Current Workspace</h1>
          </div>
          <div className="ml-auto flex items-center gap-4">
            <div className="flex items-center gap-2 text-xs text-on-surface-variant">
              <span className="h-2 w-2 rounded-full bg-primary" />
              <span>View: Recent Active</span>
            </div>
            <div className="h-4 w-px bg-outline-variant/30" />
            <div className="flex items-center gap-2 text-xs text-on-surface-variant">
              <span className="h-2 w-2 rounded-full bg-secondary" />
              <span>Layout: Hybrid</span>
            </div>
            <div className="h-4 w-px bg-outline-variant/30" />
            <div className="flex items-center gap-2 text-xs text-on-surface-variant">
              <span className="h-2 w-2 animate-pulse rounded-full bg-primary" />
              <span>Focus: Analysis Cluster</span>
            </div>
            <div className="h-4 w-px bg-outline-variant/30" />
            <div className="flex gap-1">
              <div className="rounded p-1.5 text-on-surface-variant">
                <span className="material-symbols-outlined">zoom_out</span>
              </div>
              <span className="px-2 py-1.5 font-mono text-xs text-on-surface-variant">100%</span>
              <div className="rounded p-1.5 text-on-surface-variant">
                <span className="material-symbols-outlined">zoom_in</span>
              </div>
            </div>
          </div>
        </header>

        <div className="canvas-grid relative flex-1 overflow-hidden">
          <div className="absolute left-1/2 top-1/2 w-full max-w-6xl -translate-x-1/2 -translate-y-1/2 p-12">
            <svg className="pointer-events-none absolute inset-0 h-full w-full opacity-20" xmlns="http://www.w3.org/2000/svg">
              <path d="M 500 400 L 300 250" fill="none" stroke="#bac3ff" strokeDasharray="4 4" strokeWidth="1" />
              <path d="M 500 400 L 700 250" fill="none" stroke="#bac3ff" strokeDasharray="4 4" strokeWidth="1" />
              <path d="M 500 400 L 500 600" fill="none" stroke="#bac3ff" strokeDasharray="4 4" strokeWidth="1" />
              <path d="M 300 250 L 150 200" fill="none" stroke="#acabaa" strokeWidth="1" />
              <path d="M 300 250 L 150 300" fill="none" stroke="#acabaa" strokeWidth="1" />
              <path d="M 700 250 L 850 200" fill="none" stroke="#acabaa" strokeWidth="1" />
              <path d="M 500 600 L 650 700" fill="none" stroke="#acabaa" strokeWidth="1" />
            </svg>

            <div className="absolute left-12 top-12 rounded-full border border-primary/10 bg-primary/5 px-4 py-2">
              <div className="text-[10px] font-bold uppercase tracking-[0.18em] text-primary">Recent Active Set</div>
              <p className="mt-1 text-[11px] text-on-surface-variant">
                Analysis, reporting, and linked assets are centered here. Older objects remain in the
                surrounding workspace.
              </p>
            </div>

            <div className="relative h-[800px] w-full">
              <div className="absolute left-1/2 top-1/2 z-20 -translate-x-1/2 -translate-y-1/2">
                <div className="w-[17rem] border border-primary/10 bg-surface-container p-5 shadow-2xl opacity-95">
                  <div className="-mx-6 -mt-6 mb-4 bg-secondary-container px-6 py-3">
                    <span className="text-[10px] font-bold uppercase tracking-[0.2em] text-on-secondary-container">
                      Project Anchor
                    </span>
                  </div>
                  <div className="mb-4 flex items-start justify-between">
                    <h2 className="font-headline text-lg font-extrabold text-on-surface">Project Delta</h2>
                    <span
                      className="material-symbols-outlined text-primary"
                      style={{ fontVariationSettings: "'FILL' 1" }}
                    >
                      star
                    </span>
                  </div>
                  <p className="mb-5 text-sm leading-relaxed text-on-surface-variant">
                    Workspace anchor for linked analysis, reporting, and review work.
                  </p>
                  <div className="mb-6 flex flex-wrap gap-2">
                    <span className="rounded bg-surface-container-highest px-2 py-0.5 text-[10px] text-on-surface-variant">
                      v2.4.0
                    </span>
                    <span className="rounded bg-surface-container-highest px-2 py-0.5 text-[10px] text-on-surface-variant">
                      Active
                    </span>
                  </div>
                  <div className="w-full bg-surface-container-high py-2 text-center text-xs font-bold uppercase tracking-widest text-primary">
                    Inspect Scope
                  </div>
                </div>
              </div>

              <div className="absolute left-[28%] top-[24%] z-10">
                <button
                  type="button"
                  className="block w-72 rounded-md border border-primary/20 bg-surface-container p-4 text-left shadow-xl ring-1 ring-primary/10 transition-colors hover:bg-surface-container-high"
                  aria-label="Open Analysis detail"
                  title="Open Analysis detail"
                  onClick={onOpenObjectDetail}
                >
                  <div className="mb-3 flex items-center gap-2">
                    <div className="h-6 w-1.5 bg-primary" />
                    <span className="font-headline text-sm font-bold">Analysis</span>
                    <span className="ml-auto rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.14em] text-primary">
                      Active
                    </span>
                  </div>
                  <p className="mb-4 text-xs text-on-surface-variant">
                    Baseline metric derivation from historical repository data and current run outputs.
                  </p>
                  <div className="mb-4 flex flex-wrap gap-2">
                    <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-[0.14em] text-primary">
                      Current Focus
                    </span>
                    <span className="rounded-full bg-surface-container-highest px-2 py-0.5 text-[10px] text-on-surface-variant">
                      2 linked assets
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-[10px] text-tertiary">
                    <span>Status: Running</span>
                    <span className="font-mono">84%</span>
                  </div>
                  <div className="mt-2 h-1 overflow-hidden rounded-full bg-surface-container-lowest">
                    <div className="h-full w-[84%] bg-primary" />
                  </div>
                </button>
              </div>

              <div className="absolute right-[26%] top-[26%] z-10 scale-[0.98] opacity-80">
                <div className="w-64 rounded-md border border-outline-variant/15 bg-surface-container p-4 shadow-xl">
                  <div className="mb-3 flex items-center gap-2">
                    <div className="h-6 w-1.5 bg-secondary" />
                    <span className="font-headline text-sm font-bold">Reporting</span>
                  </div>
                  <p className="mb-4 text-xs text-on-surface-variant">
                    Draft generation of executive summaries and technical logs.
                  </p>
                  <div className="flex items-center justify-between text-[10px] text-secondary">
                    <span>Status: Complete</span>
                    <span className="material-symbols-outlined text-xs">check_circle</span>
                  </div>
                </div>
              </div>

              <div className="absolute bottom-[6%] left-1/2 z-10 -translate-x-1/2 scale-[0.96] opacity-72">
                <div className="w-64 rounded-md border border-outline-variant/15 bg-surface-container p-4 shadow-xl">
                  <div className="mb-3 flex items-center gap-2">
                    <div className="h-6 w-1.5 bg-error" />
                    <span className="font-headline text-sm font-bold">Review</span>
                  </div>
                  <p className="mb-4 text-xs text-on-surface-variant">
                    Stakeholder gate verification for phase-gate transition.
                  </p>
                  <div className="flex items-center justify-between text-[10px] text-error">
                    <span>Status: Pending Input</span>
                    <span className="material-symbols-outlined text-xs">error_outline</span>
                  </div>
                </div>
              </div>

              <div className="absolute left-[10%] top-[15%]">
                <div className="flex flex-col gap-2">
                  <div className="flex w-44 items-center gap-3 rounded-xl border border-outline-variant/10 bg-surface-container-highest px-4 py-2 opacity-80 shadow-lg">
                    <span className="material-symbols-outlined text-sm text-on-surface-variant">folder_zip</span>
                    <span className="truncate text-[11px] font-medium">log_files.zip</span>
                  </div>
                  <div className="flex w-44 items-center gap-3 rounded-xl border border-outline-variant/10 bg-surface-container-highest px-4 py-2 opacity-75 shadow-lg">
                    <span className="material-symbols-outlined text-sm text-on-surface-variant">description</span>
                    <span className="truncate text-[11px] font-medium">raw_data_v1.csv</span>
                  </div>
                </div>
              </div>

              <div className="absolute right-[10%] top-[15%]">
                <div className="flex w-52 items-center gap-3 rounded-xl border border-primary/15 bg-surface-container-highest px-4 py-2 shadow-lg ring-1 ring-primary/10">
                  <span className="material-symbols-outlined text-sm text-primary">picture_as_pdf</span>
                  <span className="truncate text-[11px] font-medium">summary_report.pdf</span>
                </div>
              </div>

              <div className="absolute bottom-[10%] right-[20%]">
                <div className="flex w-48 items-center gap-3 rounded-xl border border-outline-variant/10 bg-surface-container-highest px-4 py-2 opacity-70 shadow-lg">
                  <span className="material-symbols-outlined text-sm text-on-surface-variant">table_chart</span>
                  <span className="truncate text-[11px] font-medium">metrics_sheet.xls</span>
                </div>
              </div>
            </div>
          </div>

          <div className="pointer-events-none absolute bottom-20 left-20 scale-90 opacity-40 grayscale">
            <div className="w-64 border border-outline-variant/10 bg-surface-container p-5">
              <div className="mb-3 flex items-start justify-between">
                <h3 className="font-headline text-lg font-bold text-on-surface-variant">Project Beta</h3>
              </div>
              <p className="mb-4 text-xs text-on-surface-variant/70">Legacy migration task force archives.</p>
              <div className="flex gap-2">
                <div className="h-1 w-full bg-outline-variant/20" />
              </div>
            </div>
          </div>

          <div className="absolute bottom-8 right-8 flex flex-col gap-3">
            <div className="flex h-12 w-12 items-center justify-center rounded-full border border-primary/10 bg-surface-bright text-primary shadow-2xl">
              <span className="material-symbols-outlined">grid_view</span>
            </div>
            <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary text-on-primary shadow-2xl">
              <span className="material-symbols-outlined">layers</span>
            </div>
          </div>

          <div className="pointer-events-none absolute bottom-8 left-8 h-20 w-32 overflow-hidden rounded border border-outline-variant/10 bg-surface-container-low/80 p-1 backdrop-blur">
            <div className="relative h-full w-full">
              <div className="absolute left-1/2 top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-sm bg-primary/40" />
              <div className="absolute left-1/4 top-1/4 h-2 w-2 rounded-sm bg-outline-variant/40" />
              <div className="absolute right-1/4 top-1/4 h-2 w-2 rounded-sm bg-outline-variant/40" />
              <div className="absolute bottom-1 left-1/2 h-2 w-2 rounded-sm bg-outline-variant/40" />
              <div className="absolute bottom-2 left-2 h-2 w-3 rounded-sm bg-outline-variant/20" />
              <div className="absolute inset-0 border border-primary/50" />
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

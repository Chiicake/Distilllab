export default function RightInspector() {
  return (
    <aside className="w-80 bg-surface-container-low border-l border-outline-variant/10 flex flex-col hidden lg:flex">
      <div className="p-6 border-b border-outline-variant/5">
        <h3 className="font-headline text-xs font-bold uppercase tracking-[0.2em] text-on-surface-variant mb-6">
          Session Inspector
        </h3>

        <div className="space-y-6">
          <div className="p-4 bg-surface-container-high rounded-lg border border-outline-variant/10">
            <div className="flex items-center gap-2 text-primary-dim mb-2">
              <span aria-hidden="true" className="material-symbols-outlined text-sm" data-icon="info">
                info
              </span>
              <span className="text-[10px] font-bold uppercase tracking-widest">Active State</span>
            </div>

            <p className="text-xs text-on-surface leading-relaxed">
              Draft mode. Structured objects appear here once work is created or linked, not simply because they were mentioned.
            </p>
          </div>

          <div className="space-y-4">
            <div className="text-[10px] font-bold uppercase tracking-widest text-on-surface-variant/60">Metadata</div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-[9px] text-on-surface-variant/40 uppercase mb-1">Engine</div>
                <div className="text-xs text-on-surface">Distill-v4-Core</div>
              </div>

              <div>
                <div className="text-[9px] text-on-surface-variant/40 uppercase mb-1">Privacy</div>
                <div className="text-xs text-on-surface flex items-center gap-1">
                  <span aria-hidden="true" className="material-symbols-outlined text-[10px]" data-icon="lock">
                    lock
                  </span>
                  Private
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="flex-1 p-6 flex flex-col items-center justify-center opacity-20 grayscale">
        <span aria-hidden="true" className="material-symbols-outlined text-6xl mb-4" data-icon="database_off">
          database_off
        </span>
        <p className="text-[10px] uppercase tracking-widest font-bold">No Structured Objects Yet</p>
        <p className="text-[10px] text-on-surface-variant/60 mt-2 uppercase tracking-[0.14em]">
          Objects appear after work is created, not before.
        </p>
      </div>

      <div className="p-6 mt-auto">
        <button
          aria-disabled="true"
          className="w-full py-2 bg-surface-container-highest text-on-surface-variant text-xs font-medium rounded transition-colors flex items-center justify-center gap-2 opacity-70 cursor-not-allowed"
          disabled
          type="button"
        >
          <span aria-hidden="true" className="material-symbols-outlined" data-icon="export_notes">
            export_notes
          </span>
          Export Summary
        </button>
      </div>
    </aside>
  );
}

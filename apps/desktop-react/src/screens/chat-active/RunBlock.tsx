export default function RunBlock() {
  return (
    <div className="max-w-3xl mx-auto w-full">
      <div className="border-l-2 border-primary pl-6 py-2">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <span className="material-symbols-outlined text-primary text-xl" data-icon="cognition">
              cognition
            </span>
            <h3 className="font-headline font-bold text-primary tracking-wide text-sm uppercase">
              Evidence Distillation Run
            </h3>
          </div>

          <div className="flex items-center gap-4">
            <span className="text-[11px] font-medium text-primary">In Progress</span>
            <div className="w-32 h-1 bg-surface-container-highest rounded-full overflow-hidden">
              <div className="w-2/3 h-full gradient-primary animate-pulse" />
            </div>
          </div>
        </div>

        <div className="space-y-4">
          <div className="bg-primary/5 rounded-lg border border-primary/10 px-4 py-3 flex items-center justify-between">
            <div>
              <div className="text-[10px] font-bold uppercase tracking-[0.18em] text-primary mb-1">Partial Progress</div>
              <p className="text-xs text-on-surface">
                1 work item updated, 2 related assets linked, and the current object graph is now in sync with the latest
                evidence.
              </p>
            </div>

            <span className="material-symbols-outlined text-primary" data-icon="move_up">
              move_up
            </span>
          </div>

          <div className="bg-surface-container-high p-4 rounded-lg border-l-2 border-secondary flex items-start gap-4 shadow-sm">
            <span className="material-symbols-outlined text-secondary mt-0.5" data-icon="task_alt">
              task_alt
            </span>

            <div className="flex-1">
              <div className="flex justify-between items-center mb-1">
                <span className="font-label font-bold text-xs uppercase tracking-wider text-on-surface">fetch_logs</span>
                <span className="text-[10px] text-secondary font-bold uppercase">Success</span>
              </div>

              <p className="text-sm text-on-surface-variant font-body">
                Retrieved 1,240 evidence rows from the Delta analysis set. No continuity gaps found in the imported run
                history.
              </p>
            </div>
          </div>

          <div className="bg-surface-container-high p-4 rounded-lg border-l-2 border-primary flex items-start gap-4 transition-all duration-300">
            <span className="material-symbols-outlined text-primary mt-0.5 animate-spin" data-icon="sync">
              sync
            </span>

            <div className="flex-1">
              <div className="flex justify-between items-center mb-1">
                <span className="font-label font-bold text-xs uppercase tracking-wider text-on-surface">
                  derive_workitem_state
                </span>
                <span className="text-[10px] text-primary font-bold uppercase">Working...</span>
              </div>

              <div className="h-2 w-full bg-surface-container-highest rounded-full mt-3 overflow-hidden">
                <div className="h-full gradient-primary w-1/3 transition-all duration-1000" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function DraftMain() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center overflow-y-auto p-12">
      <div className="max-w-2xl w-full space-y-8 text-center">
        <div className="space-y-4">
          <div className="mb-4 inline-flex h-16 w-16 items-center justify-center rounded-full bg-surface-container-high text-primary-dim">
            <span aria-hidden="true" className="material-symbols-outlined text-4xl" data-icon="edit_note">
              edit_note
            </span>
          </div>

          <h1 className="font-headline text-4xl font-extrabold tracking-tight text-on-surface">New Session</h1>

          <p className="mx-auto max-w-md font-body leading-relaxed text-on-surface-variant">
            Start with a task, transcript, or working question. The first message creates the session. From there,
            Distilllab turns the conversation into structured work across projects, work items, and assets.
          </p>
        </div>

        <div className="inline-flex items-center gap-2 rounded-full border border-primary/10 bg-primary/5 px-4 py-2 text-[10px] font-bold uppercase tracking-[0.18em] text-primary">
          <span className="h-1.5 w-1.5 rounded-full bg-primary" />
          <span>Ready To Distill</span>
        </div>

        <div className="mt-12 grid grid-cols-2 gap-4 text-left">
          <button
            className="group rounded-xl border border-outline-variant/10 bg-surface-container p-5 transition-all hover:bg-surface-container-high"
            type="button"
          >
            <span aria-hidden="true" className="material-symbols-outlined mb-3 block text-secondary" data-icon="auto_awesome">
              auto_awesome
            </span>
            <div className="mb-1 text-sm font-bold text-on-surface">Distill A Session</div>
            <div className="text-xs leading-snug text-on-surface-variant">
              Turn a coding or research conversation into structured work items and assets.
            </div>
          </button>

          <button
            className="group rounded-xl border border-outline-variant/10 bg-surface-container p-5 transition-all hover:bg-surface-container-high"
            type="button"
          >
            <span aria-hidden="true" className="material-symbols-outlined mb-3 block text-secondary" data-icon="terminal">
              terminal
            </span>
            <div className="mb-1 text-sm font-bold text-on-surface">Extract Work Items</div>
            <div className="text-xs leading-snug text-on-surface-variant">
              Break a messy discussion into explicit tasks, constraints, and active lines of work.
            </div>
          </button>

          <button
            className="group rounded-xl border border-outline-variant/10 bg-surface-container p-5 transition-all hover:bg-surface-container-high"
            type="button"
          >
            <span aria-hidden="true" className="material-symbols-outlined mb-3 block text-secondary" data-icon="schema">
              schema
            </span>
            <div className="mb-1 text-sm font-bold text-on-surface">Shape A Project</div>
            <div className="text-xs leading-snug text-on-surface-variant">
              Create a project structure and connect the current discussion to an evolving work world.
            </div>
          </button>

          <button
            className="group rounded-xl border border-outline-variant/10 bg-surface-container p-5 transition-all hover:bg-surface-container-high"
            type="button"
          >
            <span aria-hidden="true" className="material-symbols-outlined mb-3 block text-secondary" data-icon="database">
              database
            </span>
            <div className="mb-1 text-sm font-bold text-on-surface">Create Assets</div>
            <div className="text-xs leading-snug text-on-surface-variant">
              Promote useful outputs, notes, and references into reusable assets.
            </div>
          </button>
        </div>
      </div>
    </div>
  );
}

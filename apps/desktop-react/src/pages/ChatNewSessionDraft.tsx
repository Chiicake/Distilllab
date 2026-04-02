import LeftRail from '../components/shell/LeftRail';
import RightInspector from '../components/shell/RightInspector';
import TopNav from '../components/shell/TopNav';

type ChatNewSessionDraftProps = {
  onEnterActiveRun: () => void;
};

export default function ChatNewSessionDraft({ onEnterActiveRun }: ChatNewSessionDraftProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TopNav />

      <div className="flex flex-1 overflow-hidden">
        <LeftRail onOpenPreviewRun={onEnterActiveRun} />

        <main className="relative flex flex-1 flex-col bg-surface">
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

          <div className="bg-gradient-to-t from-surface to-transparent p-6">
            <div className="relative mx-auto max-w-4xl">
              <div className="overflow-hidden rounded-2xl bg-surface-container-high shadow-2xl transition-all focus-within:ring-1 focus-within:ring-primary/30">
                <textarea
                  aria-label="Describe the work you want to distill into structure"
                  className="min-h-[64px] w-full resize-none border-none bg-transparent p-5 font-body text-on-surface placeholder:text-on-surface-variant/40 focus:ring-0"
                  placeholder="Describe the work you want to distill into structure..."
                  rows={1}
                />

                <div className="flex items-center justify-between bg-surface-container-highest/50 px-5 py-3">
                  <div className="flex gap-4">
                    <button
                      className="flex items-center gap-1 text-xs text-on-surface-variant transition-colors hover:text-on-surface"
                      type="button"
                    >
                      <span aria-hidden="true" className="material-symbols-outlined" data-icon="attach_file">
                        attach_file
                      </span>
                      Attach
                    </button>

                    <button
                      className="flex items-center gap-1 text-xs text-on-surface-variant transition-colors hover:text-on-surface"
                      type="button"
                    >
                      <span aria-hidden="true" className="material-symbols-outlined" data-icon="memory">
                        memory
                      </span>
                      Context
                    </button>
                  </div>

                  <button
                    className="flex items-center gap-2 rounded-lg bg-primary px-4 py-1.5 text-xs font-bold uppercase tracking-widest text-on-primary transition-all hover:brightness-110"
                    type="button"
                  >
                    Send
                    <span aria-hidden="true" className="material-symbols-outlined" data-icon="arrow_forward">
                      arrow_forward
                    </span>
                  </button>
                </div>
              </div>

              <div className="mt-3 text-center text-[10px] uppercase tracking-wide text-on-surface-variant/40">
                Press <kbd className="rounded border border-outline-variant/20 bg-surface px-1.5 py-0.5">Ctrl/Cmd</kbd> +{' '}
                <kbd className="rounded border border-outline-variant/20 bg-surface px-1.5 py-0.5">Enter</kbd> to send
              </div>
            </div>
          </div>
        </main>

        <RightInspector />
      </div>
    </div>
  );
}

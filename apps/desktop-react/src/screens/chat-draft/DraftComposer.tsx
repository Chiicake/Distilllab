export default function DraftComposer() {
  return (
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
  );
}

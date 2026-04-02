type ActiveLeftRailProps = {
  onReturnToDraft: () => void;
};

export default function ActiveLeftRail({ onReturnToDraft }: ActiveLeftRailProps) {
  return (
    <aside className="bg-[#191a1a] text-[#bac3ff] font-['Inter'] text-sm docked h-full left-0 w-64 no-border bg-[#191a1a] flat no shadows flex flex-col h-full py-6 px-4 gap-4 border-r border-outline-variant/10">
      <div className="mb-4">
        <button
          className="w-full py-3 px-4 bg-gradient-to-br from-primary to-primary-container text-on-primary font-bold rounded-lg flex items-center justify-center gap-2 hover:opacity-90 transition-all uppercase tracking-widest text-xs"
          onClick={onReturnToDraft}
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="add">
            add
          </span>
          New Session
        </button>
      </div>

      <div className="flex-1 overflow-y-auto space-y-1">
        <div className="px-2 py-3 text-[10px] uppercase tracking-widest text-on-surface-variant/40 font-bold mb-1 mt-2">
          Recent Sessions
        </div>

        <div className="bg-[#1f2020] text-[#f3faff] rounded-md px-3 py-3 transition-all duration-300 border border-primary/10">
          <div className="flex items-center gap-3 mb-1.5">
            <span className="material-symbols-outlined text-primary" data-icon="chat_bubble">
              chat_bubble
            </span>
            <span className="truncate font-semibold">Project Delta Analysis</span>
          </div>
          <p className="text-[11px] text-on-surface-variant leading-relaxed">
            Active session preview. Timeline, run state, and inspector content are visible in the main workspace.
          </p>
          <div className="mt-3 flex items-center gap-2 text-[10px] uppercase tracking-[0.16em] text-primary font-bold">
            <span className="w-1.5 h-1.5 rounded-full bg-primary" />
            <span>Active Run</span>
          </div>
        </div>

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="chat_bubble">
            chat_bubble
          </span>
          <span className="truncate">Q4 Logistics Audit</span>
        </button>

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="history">
            history
          </span>
          <span className="truncate">Sprint Memory Review</span>
        </button>

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="chat_bubble">
            chat_bubble
          </span>
          <span className="truncate">Interface Refactor Notes</span>
        </button>
      </div>

      <div className="pt-6 border-t border-outline-variant/10 space-y-2">
        <button className="text-[#acabaa] opacity-60 hover:opacity-100 flex items-center gap-3 px-3 py-1.5 transition-all" type="button">
          <span className="material-symbols-outlined" data-icon="help">
            help
          </span>
          Help
        </button>

        <button className="text-[#acabaa] opacity-60 hover:opacity-100 flex items-center gap-3 px-3 py-1.5 transition-all" type="button">
          <span className="material-symbols-outlined" data-icon="sensors">
            sensors
          </span>
          Status
        </button>
      </div>
    </aside>
  );
}

type LeftRailProps = {
  onOpenPreviewRun: () => void;
};

export default function LeftRail({ onOpenPreviewRun }: LeftRailProps) {
  return (
    <aside className="bg-[#191a1a] text-[#bac3ff] font-['Inter'] text-sm docked h-full left-0 w-64 no-border bg-[#191a1a] flat no shadows flex flex-col h-full py-6 px-4 gap-4 border-r border-outline-variant/10">
      <div className="mb-4">
        <button
          className="w-full py-3 px-4 bg-gradient-to-br from-primary to-primary-container text-on-primary font-bold rounded-lg flex items-center justify-center gap-2 hover:opacity-90 transition-all uppercase tracking-widest text-xs"
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

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          onClick={onOpenPreviewRun}
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="filter_list">
            filter_list
          </span>
          <span className="truncate">Data Filtering Logic</span>
        </button>

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="account_tree">
            account_tree
          </span>
          <span className="truncate">Workflow Schema v2</span>
        </button>

        <button
          className="text-[#acabaa] opacity-60 hover:bg-[#1f2020] hover:opacity-100 rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300"
          type="button"
        >
          <span className="material-symbols-outlined" data-icon="history">
            history
          </span>
          <span className="truncate">Previous Explorations</span>
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

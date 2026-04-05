import type { ChatSessionSummary } from '../../chat/types';

type ActiveLeftRailProps = {
  onReturnToDraft: () => void;
  onOpenSession: (sessionId: string) => Promise<void>;
  activeSessionId?: string | null;
  sessions: ChatSessionSummary[];
};

export default function ActiveLeftRail({
  onReturnToDraft,
  onOpenSession,
  activeSessionId,
  sessions,
}: ActiveLeftRailProps) {
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

        {sessions.slice(0, 6).map((session) => {
          const isActive = session.sessionId === activeSessionId;

          return (
          <button
            key={session.sessionId}
            className={`w-full rounded-md px-3 py-3 text-left transition-all duration-300 hover:bg-[#1f2020] hover:opacity-100 ${
              isActive
                ? 'border border-primary/10 bg-[#1f2020] text-[#f3faff]'
                : 'text-[#acabaa] opacity-60'
            }`}
            onClick={() => {
              void onOpenSession(session.sessionId);
            }}
            type="button"
          >
            <div className="mb-1.5 flex items-center gap-3">
              <span className={`material-symbols-outlined ${isActive ? 'text-primary' : ''}`} data-icon="chat_bubble">
                chat_bubble
              </span>
              <span className="truncate font-semibold">{session.title}</span>
            </div>
            <p className="text-[11px] leading-relaxed text-on-surface-variant">{session.statusLabel}</p>
            {isActive ? (
              <div className="mt-3 flex items-center gap-2 text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
                <span className="h-1.5 w-1.5 rounded-full bg-primary" />
                <span>Live Session</span>
              </div>
            ) : null}
          </button>
        )})}
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

import type { ChatSessionSummary } from '../../chat/types';

type LeftRailProps = {
  activeSessionId?: string | null;
  sessions: ChatSessionSummary[];
  onOpenSession: (sessionId: string) => Promise<void>;
};

export default function LeftRail({ activeSessionId, sessions, onOpenSession }: LeftRailProps) {
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

        {sessions.length === 0 ? (
          <div className="rounded-md border border-outline-variant/10 px-3 py-3 text-xs text-on-surface-variant/60">
            No real sessions yet.
          </div>
        ) : null}

        {sessions.map((session) => (
          <button
            key={session.sessionId}
            className={`rounded-md flex items-center gap-3 px-3 py-2.5 transition-all duration-300 w-full text-left hover:bg-[#1f2020] hover:opacity-100 ${
              session.sessionId === activeSessionId
                ? 'border border-primary/10 bg-[#1f2020] text-[#f3faff] opacity-100'
                : 'text-[#acabaa] opacity-60'
            }`}
            onClick={() => {
              void onOpenSession(session.sessionId);
            }}
            type="button"
          >
            <span
              className={`material-symbols-outlined ${session.sessionId === activeSessionId ? 'text-primary' : ''}`}
              data-icon="chat_bubble"
            >
              chat_bubble
            </span>
            <span className="truncate">{session.title}</span>
          </button>
        ))}
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

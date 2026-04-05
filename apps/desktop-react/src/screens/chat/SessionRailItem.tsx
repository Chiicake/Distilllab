import type { ChatSessionSummary } from '../../chat/types';
import SessionActionMenu from './SessionActionMenu';

type SessionRailItemProps = {
  session: ChatSessionSummary;
  active: boolean;
  compact?: boolean;
  onOpen: (sessionId: string) => void;
  onRename: (sessionId: string, currentManualTitle: string | null, currentTitle: string) => void;
  onDelete: (sessionId: string, title: string) => void;
  onTogglePin: (sessionId: string, pinned: boolean) => void;
};

export default function SessionRailItem({
  session,
  active,
  compact = false,
  onOpen,
  onRename,
  onDelete,
  onTogglePin,
}: SessionRailItemProps) {
  return (
    <div className="relative">
      <div
        className={`group flex w-full items-start gap-2 rounded-md transition-all duration-300 hover:bg-[#1f2020] hover:opacity-100 ${
          active
            ? 'border border-primary/10 bg-[#1f2020] text-[#f3faff] opacity-100'
            : 'text-[#acabaa] opacity-60'
        } ${compact ? 'px-3 py-3' : 'px-3 py-2.5'}`}
      >
        <button className="min-w-0 flex-1 text-left" onClick={() => onOpen(session.sessionId)} type="button">
          <div className={`mb-1.5 flex items-center gap-3 ${compact ? '' : 'mb-0'}`}>
            <span className={`material-symbols-outlined ${active ? 'text-primary' : ''}`} data-icon="chat_bubble">
              chat_bubble
            </span>
            <span className="truncate font-semibold">{session.title}</span>
            {session.pinned ? (
              <span className="material-symbols-outlined text-[14px] text-primary" data-icon="keep">
                keep
              </span>
            ) : null}
          </div>
          {compact ? <p className="text-[11px] leading-relaxed text-on-surface-variant">{session.statusLabel}</p> : null}
          {compact && active ? (
            <div className="mt-3 flex items-center gap-2 text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
              <span className="h-1.5 w-1.5 rounded-full bg-primary" />
              <span>Live Session</span>
            </div>
          ) : null}
        </button>

        <SessionActionMenu
          active={active}
          onDelete={() => onDelete(session.sessionId, session.title)}
          onRename={() => onRename(session.sessionId, session.manualTitle ?? null, session.title)}
          onTogglePin={() => onTogglePin(session.sessionId, !(session.pinned ?? false))}
          pinned={Boolean(session.pinned)}
        />
      </div>
    </div>
  );
}

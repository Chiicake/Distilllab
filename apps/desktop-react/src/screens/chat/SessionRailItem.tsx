import { useEffect, useRef, useState } from 'react';

import type { ChatSessionSummary } from '../../chat/types';

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
  const [menuOpen, setMenuOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };

    window.addEventListener('mousedown', handlePointerDown);
    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
    };
  }, [menuOpen]);

  return (
    <div className="relative" ref={containerRef}>
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

        <div className="relative tauri-no-drag shrink-0">
          <button
            aria-label="Session actions"
            className={`rounded-md p-1 transition-colors ${
              menuOpen || active ? 'text-[#dbe1ff]' : 'text-[#8f95bf] opacity-0 group-hover:opacity-100'
            } hover:bg-[#2a2b2b] hover:text-[#f3faff]`}
            onClick={(event) => {
              event.stopPropagation();
              setMenuOpen((previous) => !previous);
            }}
            type="button"
          >
            <span className="material-symbols-outlined text-[18px]" data-icon="more_horiz">
              more_horiz
            </span>
          </button>

          {menuOpen ? (
            <div className="absolute right-0 top-9 z-50 min-w-[160px] rounded-lg border border-outline-variant/20 bg-[#171818] p-1 shadow-[0_12px_32px_rgba(0,0,0,0.35)]">
              <button
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs text-on-surface hover:bg-[#232424]"
                onClick={() => {
                  setMenuOpen(false);
                  onRename(session.sessionId, session.manualTitle ?? null, session.title);
                }}
                type="button"
              >
                <span className="material-symbols-outlined text-[16px]">edit</span>
                <span>Rename</span>
              </button>
              <button
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs text-on-surface hover:bg-[#232424]"
                onClick={() => {
                  setMenuOpen(false);
                  onTogglePin(session.sessionId, !(session.pinned ?? false));
                }}
                type="button"
              >
                <span className="material-symbols-outlined text-[16px]">keep</span>
                <span>{session.pinned ? 'Unpin' : 'Pin to top'}</span>
              </button>
              <button
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-xs text-[#ffb4b4] hover:bg-[#2b2020]"
                onClick={() => {
                  setMenuOpen(false);
                  onDelete(session.sessionId, session.title);
                }}
                type="button"
              >
                <span className="material-symbols-outlined text-[16px]">delete</span>
                <span>Delete</span>
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

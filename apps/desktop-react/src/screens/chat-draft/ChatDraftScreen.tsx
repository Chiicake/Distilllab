import { useChat } from '../../chat/ChatProvider';
import DraftComposer from './DraftComposer';
import DraftMain from './DraftMain';
import LeftRail from './LeftRail';
import RightInspector from './RightInspector';

type ChatDraftScreenProps = {
  onEnterActiveRun: (sessionId: string) => void;
};

export default function ChatDraftScreen({ onEnterActiveRun }: ChatDraftScreenProps) {
  const { state, deleteSession, openSession, pinSession, renameSession, sendFirstMessage } = useChat();

  return (
    <div className="flex flex-1 overflow-hidden">
      <LeftRail
        activeSessionId={state.sessionId}
        onDeleteSession={(sessionId, title) => {
          const confirmed = window.confirm(`Delete session "${title}"?`);
          if (!confirmed) {
            return;
          }
          void deleteSession(sessionId);
        }}
        onOpenSession={async (sessionId) => {
          await openSession(sessionId);
          onEnterActiveRun(sessionId);
        }}
        onRenameSession={(sessionId, currentManualTitle, currentTitle) => {
          const nextTitle = window.prompt('Rename session', currentManualTitle ?? currentTitle);
          if (nextTitle === null) {
            return;
          }
          void renameSession(sessionId, nextTitle.trim() ? nextTitle : null);
        }}
        onTogglePinSession={(sessionId, pinned) => {
          void pinSession(sessionId, pinned);
        }}
        sessions={state.sessions}
      />

      <main className="relative flex min-w-0 flex-1 flex-col bg-surface">
        <DraftMain />
        <DraftComposer
          errorText={state.errorText}
          isStreaming={state.isStreaming}
          onSend={async (message) => {
            const sessionId = await sendFirstMessage(message);
            if (sessionId) {
              onEnterActiveRun(sessionId);
            }
          }}
        />
      </main>

      <RightInspector />
    </div>
  );
}

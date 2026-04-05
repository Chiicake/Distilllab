import { useChat } from '../../chat/ChatProvider';
import DraftComposer from './DraftComposer';
import DraftMain from './DraftMain';
import LeftRail from './LeftRail';
import RightInspector from './RightInspector';

type ChatDraftScreenProps = {
  onEnterActiveRun: (sessionId: string) => void;
};

export default function ChatDraftScreen({ onEnterActiveRun }: ChatDraftScreenProps) {
  const { state, openSession, sendFirstMessage } = useChat();

  return (
    <div className="flex flex-1 overflow-hidden">
      <LeftRail
        activeSessionId={state.sessionId}
        onOpenSession={async (sessionId) => {
          await openSession(sessionId);
          onEnterActiveRun(sessionId);
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

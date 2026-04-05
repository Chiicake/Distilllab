import { useEffect } from 'react';

import { useChat } from '../../chat/ChatProvider';
import ActiveComposer from './ActiveComposer';
import ActiveInspector from './ActiveInspector';
import ActiveLeftRail from './ActiveLeftRail';
import TimelineFeed from './TimelineFeed';
import TimelineHeader from './TimelineHeader';

type ChatActiveScreenProps = {
  onReturnToDraft: () => void;
  onSelectSession: (sessionId: string) => void;
  sessionId?: string;
};

export default function ChatActiveScreen({
  onReturnToDraft,
  onSelectSession,
  sessionId,
}: ChatActiveScreenProps) {
  const { deleteSession, openSession, pinSession, renameSession, sendFollowUpMessage, state } = useChat();

  useEffect(() => {
    if (sessionId && sessionId !== state.sessionId) {
      void openSession(sessionId);
    }
  }, [openSession, sessionId, state.sessionId]);

  return (
    <div className="flex flex-1 overflow-hidden">
        <ActiveLeftRail
          activeSessionId={state.sessionId}
          onDeleteSession={(nextSessionId, title) => {
            const confirmed = window.confirm(`Delete session "${title}"?`);
            if (!confirmed) {
              return;
            }

            void (async () => {
              await deleteSession(nextSessionId);
              if (state.sessionId === nextSessionId) {
                onReturnToDraft();
              }
            })();
          }}
          onOpenSession={async (nextSessionId) => {
            onSelectSession(nextSessionId);
            await openSession(nextSessionId);
          }}
          onRenameSession={(nextSessionId, currentManualTitle, currentTitle) => {
            const nextTitle = window.prompt('Rename session', currentManualTitle ?? currentTitle);
            if (nextTitle === null) {
              return;
            }

            void renameSession(nextSessionId, nextTitle.trim() ? nextTitle : null);
          }}
          onReturnToDraft={onReturnToDraft}
          onTogglePinSession={(nextSessionId, pinned) => {
            void pinSession(nextSessionId, pinned);
          }}
          sessions={state.sessions}
        />
      <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <TimelineHeader activeRunLabel={state.activeRunLabel} sessionTitle={state.sessionTitle} />

        <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
          <TimelineFeed errorText={state.errorText} messages={state.messages} />
          <ActiveComposer
            isStreaming={state.isStreaming}
            onSend={async (message) => {
              await sendFollowUpMessage(message);
            }}
          />
        </div>
      </main>

      <ActiveInspector
        decisionSummary={state.decisionSummary}
        liveAssistantText={state.liveAssistantText}
        lastRunStatus={state.lastRunStatus}
        lastToolStatus={state.lastToolStatus}
        streamStatuses={state.streamStatuses}
      />
    </div>
  );
}

import { useEffect } from 'react';

import { useChat } from '../../chat/ChatProvider';
import ActiveComposer from './ActiveComposer';
import ActiveInspector from './ActiveInspector';
import ActiveLeftRail from './ActiveLeftRail';
import TimelineFeed from './TimelineFeed';
import TimelineHeader from './TimelineHeader';

type ChatActiveScreenProps = {
  onRequestDeleteSession: (sessionId: string, title: string) => void;
  onRequestRenameSession: (sessionId: string, currentTitle: string) => void;
  onReturnToDraft: () => void;
  onSelectSession: (sessionId: string) => void;
  sessionId?: string;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

export default function ChatActiveScreen({
  onRequestDeleteSession,
  onRequestRenameSession,
  onReturnToDraft,
  onSelectSession,
  sessionId,
  showLeftSidebar,
  showRightSidebar,
}: ChatActiveScreenProps) {
  const { openSession, pinSession, sendFollowUpMessage, state } = useChat();
  const currentRun = [...state.messages]
    .reverse()
    .find(
      (message) =>
        message.kind === 'run'
        && message.runMeta
        && (message.runMeta.state === 'running' || message.runMeta.state === 'pending'),
    )?.runMeta ?? null;

  useEffect(() => {
    if (sessionId && sessionId !== state.sessionId) {
      void openSession(sessionId);
    }
  }, [openSession, sessionId, state.sessionId]);

  return (
    <div className="flex flex-1 overflow-hidden">
      {showLeftSidebar ? (
        <ActiveLeftRail
          activeSessionId={state.sessionId}
          onDeleteSession={(nextSessionId, title) => {
            onRequestDeleteSession(nextSessionId, title);
          }}
          onOpenSession={async (nextSessionId) => {
            onSelectSession(nextSessionId);
            await openSession(nextSessionId);
          }}
          onRenameSession={(nextSessionId, currentManualTitle, currentTitle) => {
            onRequestRenameSession(nextSessionId, currentManualTitle ?? currentTitle);
          }}
          onReturnToDraft={onReturnToDraft}
          onTogglePinSession={(nextSessionId, pinned) => {
            void pinSession(nextSessionId, pinned);
          }}
          sessions={state.sessions}
        />
      ) : null}
      <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        <TimelineHeader activeRunLabel={state.activeRunLabel} currentRun={currentRun} sessionTitle={state.sessionTitle} />

        <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
          <TimelineFeed errorText={state.errorText} messages={state.messages} />
          <ActiveComposer
            isStreaming={state.isStreaming}
            onSend={async (message, attachmentPaths) => {
              await sendFollowUpMessage(message, attachmentPaths);
            }}
          />
        </div>
      </main>

      {showRightSidebar ? (
        <ActiveInspector
          currentRun={currentRun}
          decisionSummary={state.decisionSummary}
          liveAssistantText={state.liveAssistantText}
          lastRunStatus={state.lastRunStatus}
          lastToolStatus={state.lastToolStatus}
          streamStatuses={state.streamStatuses}
        />
      ) : null}
    </div>
  );
}

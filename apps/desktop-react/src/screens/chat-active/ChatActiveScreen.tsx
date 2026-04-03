import { useEffect } from 'react';

import { useChat } from '../../chat/ChatProvider';
import ActiveComposer from './ActiveComposer';
import ActiveInspector from './ActiveInspector';
import ActiveLeftRail from './ActiveLeftRail';
import TimelineFeed from './TimelineFeed';
import TimelineHeader from './TimelineHeader';

type ChatActiveScreenProps = {
  onReturnToDraft: () => void;
  sessionId?: string;
};

export default function ChatActiveScreen({ onReturnToDraft, sessionId }: ChatActiveScreenProps) {
  const { openSession, sendFollowUpMessage, state } = useChat();

  useEffect(() => {
    if (sessionId && sessionId !== state.sessionId) {
      void openSession(sessionId);
    }
  }, [openSession, sessionId, state.sessionId]);

  return (
    <div className="flex flex-1 overflow-hidden">
      <ActiveLeftRail onReturnToDraft={onReturnToDraft} sessions={state.sessions} />

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

import ActiveComposer from '../components/chat-active/ActiveComposer';
import ActiveInspector from '../components/chat-active/ActiveInspector';
import ActiveLeftRail from '../components/chat-active/ActiveLeftRail';
import TimelineFeed from '../components/chat-active/TimelineFeed';
import TimelineHeader from '../components/chat-active/TimelineHeader';
import TopNav from '../components/shell/TopNav';

type ChatActiveRunProps = {
  onReturnToDraft: () => void;
};

export default function ChatActiveRun({ onReturnToDraft }: ChatActiveRunProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-surface text-on-surface">
      <TopNav />

      <div className="flex flex-1 overflow-hidden">
        <ActiveLeftRail onReturnToDraft={onReturnToDraft} />

        <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
          <TimelineHeader />

          <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
            <TimelineFeed />
            <ActiveComposer />
          </div>
        </main>

        <ActiveInspector />
      </div>
    </div>
  );
}

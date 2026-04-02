import ActiveComposer from './ActiveComposer';
import ActiveInspector from './ActiveInspector';
import ActiveLeftRail from './ActiveLeftRail';
import TimelineFeed from './TimelineFeed';
import TimelineHeader from './TimelineHeader';

type ChatActiveScreenProps = {
  onReturnToDraft: () => void;
};

export default function ChatActiveScreen({ onReturnToDraft }: ChatActiveScreenProps) {
  return (
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
  );
}

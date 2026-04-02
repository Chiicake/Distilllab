import DraftComposer from './DraftComposer';
import DraftMain from './DraftMain';
import LeftRail from './LeftRail';
import RightInspector from './RightInspector';

type ChatDraftScreenProps = {
  onEnterActiveRun: () => void;
};

export default function ChatDraftScreen({ onEnterActiveRun }: ChatDraftScreenProps) {
  return (
    <div className="flex flex-1 overflow-hidden">
      <LeftRail onOpenPreviewRun={onEnterActiveRun} />

      <main className="relative flex min-w-0 flex-1 flex-col bg-surface">
        <DraftMain />
        <DraftComposer />
      </main>

      <RightInspector />
    </div>
  );
}

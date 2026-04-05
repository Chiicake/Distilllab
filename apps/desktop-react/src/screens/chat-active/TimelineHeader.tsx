import type { RunCardMeta } from '../../chat/types';

type TimelineHeaderProps = {
  sessionTitle: string;
  activeRunLabel: string | null;
  currentRun?: RunCardMeta | null;
};

function formatRunTypeLabel(runType: string | null | undefined) {
  if (!runType) {
    return 'Run';
  }

  return runType
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export default function TimelineHeader({ sessionTitle, activeRunLabel, currentRun }: TimelineHeaderProps) {
  return (
    <header className="h-14 px-8 flex items-center justify-between bg-surface-container/30 backdrop-blur-sm z-10">
      <h2 className="font-headline font-bold text-on-surface tracking-tight">{sessionTitle}</h2>

      <div className="flex gap-4">
        {currentRun ? (
          <span className="label-sm rounded bg-primary/10 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-primary">
            {formatRunTypeLabel(currentRun.runType)}
          </span>
        ) : null}
        {activeRunLabel ? (
          <span className="label-sm rounded bg-secondary-container/30 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-secondary">
            {activeRunLabel}
          </span>
        ) : null}
      </div>
    </header>
  );
}

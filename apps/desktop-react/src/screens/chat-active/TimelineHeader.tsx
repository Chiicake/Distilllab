type TimelineHeaderProps = {
  sessionTitle: string;
  activeRunLabel: string | null;
};

export default function TimelineHeader({ sessionTitle, activeRunLabel }: TimelineHeaderProps) {
  return (
    <header className="h-14 px-8 flex items-center justify-between bg-surface-container/30 backdrop-blur-sm z-10">
      <h2 className="font-headline font-bold text-on-surface tracking-tight">{sessionTitle}</h2>

      <div className="flex gap-4">
        {activeRunLabel ? (
          <span className="label-sm rounded bg-secondary-container/30 px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-secondary">
            {activeRunLabel}
          </span>
        ) : null}
      </div>
    </header>
  );
}

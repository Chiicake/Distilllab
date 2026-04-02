export default function TimelineHeader() {
  return (
    <header className="h-14 px-8 flex items-center justify-between bg-surface-container/30 backdrop-blur-sm z-10">
      <h2 className="font-headline font-bold text-on-surface tracking-tight">Project Delta Analysis</h2>

      <div className="flex gap-4">
        <span className="label-sm px-2 py-0.5 rounded bg-secondary-container/30 text-secondary text-[10px] font-bold uppercase tracking-widest">
          Active Run
        </span>
      </div>
    </header>
  );
}

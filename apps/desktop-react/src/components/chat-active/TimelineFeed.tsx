import RunBlock from './RunBlock';

export default function TimelineFeed() {
  return (
    <div className="flex-1 overflow-y-auto px-8 py-10 space-y-12 no-scrollbar">
      <div className="max-w-3xl mx-auto flex flex-col items-end w-full group">
        <div className="flex items-start gap-4">
          <div className="text-right">
            <p className="text-on-surface text-lg font-body leading-relaxed max-w-xl">
              Analyze the latest results for Project Delta.
            </p>
          </div>

          <div className="w-0.5 h-12 bg-primary rounded-full mt-1" />
        </div>
      </div>

      <div className="max-w-3xl mx-auto flex flex-col items-start w-full">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-6 h-6 rounded-full bg-surface-container-high border border-primary/20 flex items-center justify-center">
            <span className="material-symbols-outlined text-[14px] text-primary" data-icon="smart_toy">
              smart_toy
            </span>
          </div>
          <span className="text-[10px] font-bold tracking-widest uppercase text-on-surface-variant font-label">
            Agent / Analyst
          </span>
        </div>

        <div className="bg-surface-container-low p-6 rounded-xl border-none max-w-xl">
          <p className="text-on-surface-variant text-md font-body leading-relaxed">
            Starting the analysis now. I am gathering the latest evidence, updating active work items, and checking which
            existing assets should be linked into this run.
          </p>
        </div>
      </div>

      <RunBlock />
    </div>
  );
}

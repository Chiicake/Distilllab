export default function CanvasScreen() {
  return (
    <div className="flex min-w-0 flex-1 items-center justify-center bg-surface px-6 py-10">
      <div className="max-w-xl space-y-4 text-center">
        <h1 className="font-headline text-3xl font-extrabold text-on-surface">Canvas</h1>
        <p className="text-sm leading-relaxed text-on-surface-variant">
          Canvas preview placeholder. This screen exists so the app architecture has an explicit
          non-chat workspace before the real canvas UI lands.
        </p>
      </div>
    </div>
  );
}

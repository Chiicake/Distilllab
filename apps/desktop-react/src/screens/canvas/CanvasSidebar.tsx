type CanvasSidebarProps = {
  activeView: 'recent-active' | 'all-objects' | 'filters' | 'search' | 'outline';
  showInventory?: boolean;
};

const viewItems = [
  { key: 'recent-active' as const, icon: 'flare', label: 'Recent Active' },
  { key: 'all-objects' as const, icon: 'deployed_code', label: 'All Objects' },
  { key: 'filters' as const, icon: 'filter_list', label: 'Filters' },
  { key: 'search' as const, icon: 'manage_search', label: 'Search' },
  { key: 'outline' as const, icon: 'account_tree', label: 'Outline' },
];

const inventorySections = [
  {
    name: 'Projects',
    accentClassName: 'text-secondary',
    items: [
      { label: 'Project Delta', className: 'text-on-surface' },
      { label: 'Project Beta', className: 'text-on-surface-variant/50' },
    ],
  },
  {
    name: 'WorkItems',
    accentClassName: 'text-tertiary',
    items: [
      { label: 'Analysis', className: 'text-primary font-semibold' },
      { label: 'Reporting', className: 'text-on-surface-variant' },
      { label: 'Review', className: 'text-on-surface-variant/60' },
    ],
  },
  {
    name: 'Assets',
    accentClassName: 'text-on-surface-variant',
    items: [
      { label: 'summary_report.pdf', className: 'text-primary/90' },
      { label: 'log_files.zip', className: 'text-on-surface-variant' },
    ],
  },
];

export default function CanvasSidebar({ activeView, showInventory = false }: CanvasSidebarProps) {
  return (
    <aside className="z-20 flex h-full w-64 shrink-0 flex-col gap-4 overflow-y-auto bg-[#191a1a] px-4 py-6 font-body text-sm text-[#bac3ff]">
      <div className="flex flex-col gap-1">
        <div className="px-3 py-1 text-[10px] font-bold uppercase tracking-[0.18em] text-on-surface-variant/40">
          View
        </div>
        {viewItems.map((item) => {
          const isActive = item.key === activeView;
          return (
            <div
              key={item.key}
              className={`flex items-center gap-3 rounded-md px-3 py-2 transition-all duration-300 ${
                isActive
                  ? 'bg-[#1f2020] text-[#f3faff]'
                  : 'text-[#acabaa] opacity-60'
              }`}
            >
              <span className="material-symbols-outlined">{item.icon}</span>
              <span>{item.label}</span>
              {isActive && (
                <span className="ml-auto text-[10px] font-bold uppercase tracking-[0.16em] text-primary">
                  Default
                </span>
              )}
            </div>
          );
        })}
      </div>

      {showInventory && (
        <div className="mt-8">
          <p className="mb-4 px-3 text-[10px] uppercase tracking-widest text-on-surface-variant">Inventory</p>
          <div className="space-y-4 px-3">
            {inventorySections.map((section) => (
              <div key={section.name}>
                <div className="mb-2 flex items-center justify-between">
                  <span className={`text-xs font-semibold ${section.accentClassName}`}>{section.name}</span>
                  <span className="material-symbols-outlined text-xs text-on-surface-variant/40">
                    keyboard_arrow_down
                  </span>
                </div>
                <ul className="space-y-2 border-l border-outline-variant/30 pl-2 text-xs">
                  {section.items.map((item) => (
                    <li key={item.label} className={item.className}>
                      {item.label}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="mt-auto flex flex-col gap-1 border-t border-outline-variant/20 pt-4">
        <div className="flex items-center gap-3 px-3 py-2 text-[#acabaa] opacity-60">
          <span className="material-symbols-outlined">help</span>
          <span>Help</span>
        </div>
        <div className="flex items-center gap-3 px-3 py-2 text-[#acabaa] opacity-60">
          <span className="material-symbols-outlined">sensors</span>
          <span>Status</span>
        </div>
      </div>
    </aside>
  );
}

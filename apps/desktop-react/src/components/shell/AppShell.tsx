import type { ReactNode } from 'react';

import WindowResizeHandles from './WindowResizeHandles';

type AppShellProps = {
  topNav: ReactNode;
  children: ReactNode;
  onStartResize?: (
    direction: 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West',
  ) => void;
};

export default function AppShell({ topNav, children, onStartResize }: AppShellProps) {
  return (
    <div className="relative flex h-screen flex-col overflow-hidden bg-surface text-on-surface">
      {onStartResize ? <WindowResizeHandles onStartResize={onStartResize} /> : null}
      {topNav}
      <div className="flex flex-1 overflow-hidden">{children}</div>
    </div>
  );
}

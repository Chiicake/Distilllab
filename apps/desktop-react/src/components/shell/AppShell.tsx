import type { ReactNode } from 'react';

type AppShellProps = {
  topNav: ReactNode;
  children: ReactNode;
};

export default function AppShell({ topNav, children }: AppShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-surface text-on-surface">
      {topNav}
      <div className="flex flex-1 overflow-hidden">{children}</div>
    </div>
  );
}

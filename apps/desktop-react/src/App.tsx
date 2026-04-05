import { useCallback, useEffect, useState } from 'react';

import { activeChatScreen, draftScreen, type Screen } from './app-state/screen-state';
import { ChatAppearanceProvider } from './chat/ChatAppearanceProvider';
import { useChat } from './chat/ChatProvider';
import AppShell from './components/shell/AppShell';
import TopNav from './components/shell/TopNav';
import ChatActiveScreen from './screens/chat-active/ChatActiveScreen';
import SessionDeleteDialog from './screens/chat/SessionDeleteDialog';
import SessionRenameDialog from './screens/chat/SessionRenameDialog';
import ChatDraftScreen from './screens/chat-draft/ChatDraftScreen';
import CanvasScreen from './screens/canvas/CanvasScreen';
import SettingsScreen from './screens/settings/SettingsScreen';

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West';

type ResizeApi = {
  startResizeDragging: (direction: ResizeDirection) => Promise<void>;
};

type SessionRenameState = {
  sessionId: string;
  currentTitle: string;
};

type SessionDeleteState = {
  sessionId: string;
  title: string;
};

type SidebarAvailability = {
  left: boolean;
  right: boolean;
};

export default function App() {
  const [screen, setScreen] = useState<Screen>(draftScreen);
  const { deleteSession, renameSession, state: chatState, resetDraft } = useChat();
  const [resizeApi, setResizeApi] = useState<ResizeApi | null>(null);
  const [renameState, setRenameState] = useState<SessionRenameState | null>(null);
  const [deleteState, setDeleteState] = useState<SessionDeleteState | null>(null);
  const [leftSidebarOpen, setLeftSidebarOpen] = useState(true);
  const [rightSidebarOpen, setRightSidebarOpen] = useState(true);

  useEffect(() => {
    let cancelled = false;

    const setupResizeApi = async () => {
      try {
        const module = await import('@tauri-apps/api/window');
        const appWindow = module.getCurrentWindow();
        if (cancelled) {
          return;
        }

        setResizeApi({
          startResizeDragging: (direction) => appWindow.startResizeDragging(direction),
        });
      } catch {
        if (!cancelled) {
          setResizeApi(null);
        }
      }
    };

    void setupResizeApi();

    return () => {
      cancelled = true;
    };
  }, []);

  const handleStartResize = useCallback(
    (direction: ResizeDirection) => {
      if (!resizeApi) {
        return;
      }

      void resizeApi.startResizeDragging(direction).catch((error) => {
        if (typeof window !== 'undefined') {
          window.console.error('[App] start resize dragging failed', error);
        }
      });
    },
    [resizeApi],
  );

  const sidebarAvailability: SidebarAvailability = (() => {
    switch (screen.kind) {
      case 'chat-draft':
      case 'chat-active':
        return { left: true, right: true };
      case 'canvas':
        return { left: true, right: true };
      case 'settings':
        return { left: true, right: false };
    }
  })();

  let content: JSX.Element;

  switch (screen.kind) {
    case 'chat-draft':
      content = (
        <ChatDraftScreen
          showLeftSidebar={leftSidebarOpen}
          showRightSidebar={rightSidebarOpen}
          onRequestDeleteSession={(sessionId, title) => setDeleteState({ sessionId, title })}
          onEnterActiveRun={(sessionId) => setScreen({ kind: 'chat-active', sessionId })}
          onRequestRenameSession={(sessionId, currentTitle) => setRenameState({ sessionId, currentTitle })}
        />
      );
      break;
    case 'chat-active':
      content = (
        <ChatActiveScreen
          showLeftSidebar={leftSidebarOpen}
          showRightSidebar={rightSidebarOpen}
          onRequestDeleteSession={(targetSessionId, title) => setDeleteState({ sessionId: targetSessionId, title })}
          onReturnToDraft={() => {
            resetDraft();
            setScreen(draftScreen);
          }}
          onRequestRenameSession={(targetSessionId, currentTitle) =>
            setRenameState({ sessionId: targetSessionId, currentTitle })
          }
          onSelectSession={(sessionId) => setScreen({ kind: 'chat-active', sessionId })}
          sessionId={screen.sessionId ?? chatState.sessionId ?? undefined}
        />
      );
      break;
    case 'canvas':
      content = <CanvasScreen showLeftSidebar={leftSidebarOpen} showRightSidebar={rightSidebarOpen} />;
      break;
    case 'settings':
      content = (
        <SettingsScreen
          onChangeSection={(section) => setScreen({ kind: 'settings', section })}
          section={screen.section}
          showLeftSidebar={leftSidebarOpen}
        />
      );
      break;
  }

  return (
    <ChatAppearanceProvider>
      <AppShell
      onStartResize={resizeApi ? handleStartResize : undefined}
      topNav={
        <TopNav
          currentScreen={screen}
          canToggleLeftSidebar={sidebarAvailability.left}
          canToggleRightSidebar={sidebarAvailability.right}
          leftSidebarOpen={leftSidebarOpen}
          rightSidebarOpen={rightSidebarOpen}
          onOpenCanvas={() => setScreen({ kind: 'canvas' })}
          onOpenChat={() => setScreen(chatState.sessionId ? activeChatScreen : draftScreen)}
          onOpenSettings={() => setScreen({ kind: 'settings' })}
          onToggleLeftSidebar={() => {
            if (!sidebarAvailability.left) {
              return;
            }
            setLeftSidebarOpen((previous) => !previous);
          }}
          onToggleRightSidebar={() => {
            if (!sidebarAvailability.right) {
              return;
            }
            setRightSidebarOpen((previous) => !previous);
          }}
        />
      }
      >
        {content}
        <SessionRenameDialog
        currentTitle={renameState?.currentTitle ?? ''}
        onClose={() => setRenameState(null)}
        onSave={(value) => {
          if (!renameState) {
            return;
          }

          const sessionId = renameState.sessionId;
          setRenameState(null);
          void renameSession(sessionId, value);
        }}
        open={renameState != null}
        />
        <SessionDeleteDialog
        onClose={() => setDeleteState(null)}
        onDelete={() => {
          if (!deleteState) {
            return;
          }

          const deletingCurrent = chatState.sessionId === deleteState.sessionId;
          const sessionId = deleteState.sessionId;
          setDeleteState(null);
          void (async () => {
            await deleteSession(sessionId);
            if (deletingCurrent) {
              resetDraft();
              setScreen(draftScreen);
            }
          })();
        }}
        open={deleteState != null}
        sessionTitle={deleteState?.title ?? ''}
        />
      </AppShell>
    </ChatAppearanceProvider>
  );
}

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

export default function App() {
  const [screen, setScreen] = useState<Screen>(draftScreen);
  const { deleteSession, renameSession, state: chatState, resetDraft } = useChat();
  const [resizeApi, setResizeApi] = useState<ResizeApi | null>(null);
  const [renameState, setRenameState] = useState<SessionRenameState | null>(null);
  const [deleteState, setDeleteState] = useState<SessionDeleteState | null>(null);

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

  let content: JSX.Element;

  switch (screen.kind) {
    case 'chat-draft':
      content = (
        <ChatDraftScreen
          onRequestDeleteSession={(sessionId, title) => setDeleteState({ sessionId, title })}
          onEnterActiveRun={(sessionId) => setScreen({ kind: 'chat-active', sessionId })}
          onRequestRenameSession={(sessionId, currentTitle) => setRenameState({ sessionId, currentTitle })}
        />
      );
      break;
    case 'chat-active':
      content = (
        <ChatActiveScreen
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
      content = <CanvasScreen />;
      break;
    case 'settings':
      content = (
        <SettingsScreen
          onChangeSection={(section) => setScreen({ kind: 'settings', section })}
          section={screen.section}
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
          onOpenCanvas={() => setScreen({ kind: 'canvas' })}
          onOpenChat={() => setScreen(chatState.sessionId ? activeChatScreen : draftScreen)}
          onOpenSettings={() => setScreen({ kind: 'settings' })}
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

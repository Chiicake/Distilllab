import { useCallback, useEffect, useState } from 'react';

import { activeChatScreen, draftScreen, type Screen } from './app-state/screen-state';
import { useChat } from './chat/ChatProvider';
import AppShell from './components/shell/AppShell';
import TopNav from './components/shell/TopNav';
import ChatActiveScreen from './screens/chat-active/ChatActiveScreen';
import ChatDraftScreen from './screens/chat-draft/ChatDraftScreen';
import CanvasScreen from './screens/canvas/CanvasScreen';
import SettingsScreen from './screens/settings/SettingsScreen';

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West';

type ResizeApi = {
  startResizeDragging: (direction: ResizeDirection) => Promise<void>;
};

export default function App() {
  const [screen, setScreen] = useState<Screen>(draftScreen);
  const { state: chatState, resetDraft } = useChat();
  const [resizeApi, setResizeApi] = useState<ResizeApi | null>(null);

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
          onEnterActiveRun={(sessionId) => setScreen({ kind: 'chat-active', sessionId })}
        />
      );
      break;
    case 'chat-active':
      content = (
        <ChatActiveScreen
          onReturnToDraft={() => {
            resetDraft();
            setScreen(draftScreen);
          }}
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
    </AppShell>
  );
}

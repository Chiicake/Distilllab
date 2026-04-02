import { useState } from 'react';

import { activeChatScreen, draftScreen, type Screen } from './app-state/screen-state';
import AppShell from './components/shell/AppShell';
import TopNav from './components/shell/TopNav';
import ChatActiveScreen from './screens/chat-active/ChatActiveScreen';
import ChatDraftScreen from './screens/chat-draft/ChatDraftScreen';
import CanvasScreen from './screens/canvas/CanvasScreen';
import SettingsScreen from './screens/settings/SettingsScreen';

export default function App() {
  const [screen, setScreen] = useState<Screen>(draftScreen);

  let content: JSX.Element;

  switch (screen.kind) {
    case 'chat-draft':
      content = <ChatDraftScreen onEnterActiveRun={() => setScreen(activeChatScreen)} />;
      break;
    case 'chat-active':
      content = <ChatActiveScreen onReturnToDraft={() => setScreen(draftScreen)} />;
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
      topNav={
        <TopNav
          currentScreen={screen}
          onOpenCanvas={() => setScreen({ kind: 'canvas' })}
          onOpenChat={() => setScreen(draftScreen)}
          onOpenSettings={() => setScreen({ kind: 'settings' })}
        />
      }
    >
      {content}
    </AppShell>
  );
}

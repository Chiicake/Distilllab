import { useState } from 'react';

import ChatActiveRun from './pages/ChatActiveRun';
import ChatNewSessionDraft from './pages/ChatNewSessionDraft';

export default function App() {
  const [previewState, setPreviewState] = useState<'draft' | 'active-run'>('draft');

  if (previewState === 'draft') {
    return <ChatNewSessionDraft onEnterActiveRun={() => setPreviewState('active-run')} />;
  }

  return <ChatActiveRun onReturnToDraft={() => setPreviewState('draft')} />;
}

import { useState } from 'react';

import CanvasGlobalView from './CanvasGlobalView';
import CanvasObjectDetail from './CanvasObjectDetail';

type CanvasMockView =
  | { kind: 'global-view' }
  | { kind: 'object-detail'; objectId: 'analysis-an-402-delta' };

export default function CanvasScreen() {
  const [view, setView] = useState<CanvasMockView>({ kind: 'global-view' });

  const openAnalysisDetail = () => {
    setView({ kind: 'object-detail', objectId: 'analysis-an-402-delta' });
  };

  const returnToGlobalView = () => {
    setView({ kind: 'global-view' });
  };

  switch (view.kind) {
    case 'object-detail':
      return <CanvasObjectDetail onReturnToGlobalView={returnToGlobalView} />;
    case 'global-view':
      return <CanvasGlobalView onOpenObjectDetail={openAnalysisDetail} />;
  }
}

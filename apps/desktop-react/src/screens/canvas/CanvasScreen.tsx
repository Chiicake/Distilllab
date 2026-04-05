import { useState } from 'react';

import CanvasGlobalView from './CanvasGlobalView';
import CanvasObjectDetail from './CanvasObjectDetail';

type CanvasMockView =
  | { kind: 'global-view' }
  | { kind: 'object-detail'; objectId: 'analysis-an-402-delta' };

type CanvasScreenProps = {
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
};

export default function CanvasScreen({ showLeftSidebar, showRightSidebar }: CanvasScreenProps) {
  const [view, setView] = useState<CanvasMockView>({ kind: 'global-view' });

  const openAnalysisDetail = () => {
    setView({ kind: 'object-detail', objectId: 'analysis-an-402-delta' });
  };

  const returnToGlobalView = () => {
    setView({ kind: 'global-view' });
  };

  switch (view.kind) {
    case 'object-detail':
      return <CanvasObjectDetail onReturnToGlobalView={returnToGlobalView} showLeftSidebar={showLeftSidebar} showRightSidebar={showRightSidebar} />;
    case 'global-view':
      return <CanvasGlobalView onOpenObjectDetail={openAnalysisDetail} showLeftSidebar={showLeftSidebar} />;
  }
}

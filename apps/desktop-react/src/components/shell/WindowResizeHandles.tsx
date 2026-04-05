import { useCallback } from 'react';

type ResizeDirection =
  | 'East'
  | 'North'
  | 'NorthEast'
  | 'NorthWest'
  | 'South'
  | 'SouthEast'
  | 'SouthWest'
  | 'West';

type WindowResizeHandlesProps = {
  onStartResize: (direction: ResizeDirection) => void;
};

const HANDLE_DEFINITIONS: Array<{
  direction: ResizeDirection;
  className: string;
}> = [
  { direction: 'North', className: 'window-resize-handle inset-x-3 top-0 h-1 cursor-n-resize' },
  { direction: 'South', className: 'window-resize-handle inset-x-3 bottom-0 h-1 cursor-s-resize' },
  { direction: 'West', className: 'window-resize-handle left-0 top-3 bottom-3 w-1 cursor-w-resize' },
  { direction: 'East', className: 'window-resize-handle right-0 top-3 bottom-3 w-1 cursor-e-resize' },
  { direction: 'NorthWest', className: 'window-resize-handle left-0 top-0 h-3 w-3 cursor-nw-resize' },
  { direction: 'NorthEast', className: 'window-resize-handle right-0 top-0 h-3 w-3 cursor-ne-resize' },
  { direction: 'SouthWest', className: 'window-resize-handle bottom-0 left-0 h-3 w-3 cursor-sw-resize' },
  { direction: 'SouthEast', className: 'window-resize-handle bottom-0 right-0 h-3 w-3 cursor-se-resize' },
];

export default function WindowResizeHandles({ onStartResize }: WindowResizeHandlesProps) {
  const handleMouseDown = useCallback(
    (direction: ResizeDirection) => (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.button !== 0) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      onStartResize(direction);
    },
    [onStartResize],
  );

  return (
    <>
      {HANDLE_DEFINITIONS.map((handle) => (
        <div
          key={handle.direction}
          className={handle.className}
          onMouseDown={handleMouseDown(handle.direction)}
        />
      ))}
    </>
  );
}

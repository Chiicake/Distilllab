import type { ChatFontSize } from './ChatAppearanceProvider';

export function chatBodyTextClass(fontSize: ChatFontSize) {
  switch (fontSize) {
    case 'large':
      return 'text-[15px]';
    case 'medium':
      return 'text-[14px]';
    case 'small':
    default:
      return 'text-[13px]';
  }
}

export function chatSecondaryTextClass(fontSize: ChatFontSize) {
  switch (fontSize) {
    case 'large':
      return 'text-[12px]';
    case 'medium':
      return 'text-[11px]';
    case 'small':
    default:
      return 'text-[10px]';
  }
}

export function chatComposerTextClass(fontSize: ChatFontSize) {
  switch (fontSize) {
    case 'large':
      return 'text-[15px]';
    case 'medium':
      return 'text-[14px]';
    case 'small':
    default:
      return 'text-[13px]';
  }
}

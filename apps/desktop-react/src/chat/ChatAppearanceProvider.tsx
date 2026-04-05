import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

export type ChatFontSize = 'small' | 'medium' | 'large';

type ChatAppearanceContextValue = {
  chatFontSize: ChatFontSize;
  setChatFontSize: (value: ChatFontSize) => void;
};

const CHAT_FONT_SIZE_STORAGE_KEY = 'distilllab.desktop-react.chat-font-size';

const ChatAppearanceContext = createContext<ChatAppearanceContextValue | null>(null);

function isChatFontSize(value: string | null): value is ChatFontSize {
  return value === 'small' || value === 'medium' || value === 'large';
}

function resolveInitialChatFontSize(): ChatFontSize {
  if (typeof window === 'undefined') {
    return 'small';
  }

  const storedValue = window.localStorage.getItem(CHAT_FONT_SIZE_STORAGE_KEY);
  return isChatFontSize(storedValue) ? storedValue : 'small';
}

export function ChatAppearanceProvider({ children }: { children: ReactNode }) {
  const [chatFontSize, setChatFontSize] = useState<ChatFontSize>(() => resolveInitialChatFontSize());

  useEffect(() => {
    window.localStorage.setItem(CHAT_FONT_SIZE_STORAGE_KEY, chatFontSize);
  }, [chatFontSize]);

  const value = useMemo<ChatAppearanceContextValue>(
    () => ({
      chatFontSize,
      setChatFontSize,
    }),
    [chatFontSize],
  );

  return <ChatAppearanceContext.Provider value={value}>{children}</ChatAppearanceContext.Provider>;
}

export function useChatAppearance() {
  const context = useContext(ChatAppearanceContext);
  if (!context) {
    throw new Error('useChatAppearance must be used within ChatAppearanceProvider');
  }

  return context;
}

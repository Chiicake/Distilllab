import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

import { defaultLocale, isLocale, localeOptions, messages, type Locale, type MessageKey } from './messages';

const LOCALE_STORAGE_KEY = 'distilllab.desktop-react.locale';

type I18nContextValue = {
  locale: Locale;
  setLocale: (nextLocale: Locale) => void;
  t: (key: MessageKey) => string;
  localeOptions: typeof localeOptions;
};

const I18nContext = createContext<I18nContextValue | null>(null);

function resolveInitialLocale(): Locale {
  if (typeof window === 'undefined') {
    return defaultLocale;
  }

  const storedLocale = window.localStorage.getItem(LOCALE_STORAGE_KEY);

  if (storedLocale && isLocale(storedLocale)) {
    return storedLocale;
  }

  const browserLocale = window.navigator.language;

  if (browserLocale === 'zh-CN' || browserLocale.startsWith('zh')) {
    return 'zh-CN';
  }

  return defaultLocale;
}

type I18nProviderProps = {
  children: ReactNode;
};

export default function I18nProvider({ children }: I18nProviderProps) {
  const [locale, setLocale] = useState<Locale>(() => resolveInitialLocale());

  useEffect(() => {
    window.localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  }, [locale]);

  const value = useMemo<I18nContextValue>(() => {
    return {
      locale,
      setLocale,
      t: (key) => messages[locale][key],
      localeOptions,
    };
  }, [locale]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);

  if (!context) {
    throw new Error('useI18n must be used within I18nProvider');
  }

  return context;
}

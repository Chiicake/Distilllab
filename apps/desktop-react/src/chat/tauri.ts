export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export type TauriUnlisten = () => void;

export type TauriListenFn = <T>(
  eventName: string,
  handler: (event: { payload: T }) => void,
) => Promise<TauriUnlisten>;

type TauriEventApi = {
  listen?: TauriListenFn;
};

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke?: TauriInvoke;
    };
    __TAURI__?: {
      event?: {
        listen?: TauriListenFn;
      };
    };
  }
}

export function getTauriInvoke(): TauriInvoke | null {
  if (typeof window === 'undefined') {
    return null;
  }

  return window.__TAURI_INTERNALS__?.invoke ?? null;
}

export function getTauriListen(): TauriListenFn | null {
  if (typeof window === 'undefined') {
    return null;
  }

  if (window.__TAURI__?.event?.listen) {
    return window.__TAURI__.event.listen;
  }

  return null;
}

export async function loadTauriEventApi(): Promise<TauriEventApi | null> {
  if (typeof window === 'undefined') {
    return null;
  }

  if (window.__TAURI__?.event) {
    return window.__TAURI__.event;
  }

  try {
    const module = (await import('@tauri-apps/api/event')) as {
      listen?: TauriListenFn;
      default?: { listen?: TauriListenFn };
    };

    const listen = module.listen ?? module.default?.listen;
    return listen ? { listen } : null;
  } catch {
    return null;
  }
}

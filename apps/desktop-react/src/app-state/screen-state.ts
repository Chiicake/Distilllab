export type Screen =
  | { kind: 'chat-draft' }
  | { kind: 'chat-active' }
  | { kind: 'canvas' }
  | { kind: 'settings'; section?: 'workspace' | 'debug' };

export type SettingsSection = NonNullable<Extract<Screen, { kind: 'settings' }>['section']>;

export const draftScreen: Screen = { kind: 'chat-draft' };
export const activeChatScreen: Screen = { kind: 'chat-active' };

import { getTauriInvoke } from '../../chat/tauri';

export type PendingAttachment = {
  path: string;
  name: string;
};

export async function pickPendingAttachments(): Promise<PendingAttachment[]> {
  const invoke = getTauriInvoke();
  if (!invoke) {
    return [];
  }

  const raw = await invoke<string>('pick_attachments_command');
  return JSON.parse(raw) as PendingAttachment[];
}

export function mergePendingAttachments(
  existing: PendingAttachment[],
  incoming: PendingAttachment[],
): PendingAttachment[] {
  const merged = [...existing];

  for (const attachment of incoming) {
    if (!merged.some((item) => item.path === attachment.path)) {
      merged.push(attachment);
    }
  }

  return merged;
}

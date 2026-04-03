import type { ChatMessage, ChatMessageRole } from './types';

const HEADER_PATTERN = /^\[(User|Assistant|System|Tool)\]/;

function compactJsonSnippet(text: string): string {
  return text.replace(/\s+/g, ' ').trim();
}

function summarizeToolHeader(header: string, body: string): string {
  const rawHeader = header.replace(/^\[Tool\]\s*/, '').trim();
  const success = !/error|failed|failure/i.test(body);
  const status = success ? 'success' : 'error';

  const match = rawHeader.match(/^([^\(]+)(\((.*)\))?$/);
  if (!match) {
    return `${rawHeader} · ${status}`;
  }

  const toolName = match[1]?.trim() ?? rawHeader;
  const rawArgs = match[3]?.trim();

  if (!rawArgs) {
    return `${toolName} · ${status}`;
  }

  let argsPreview = rawArgs;
  try {
    const parsed = JSON.parse(rawArgs) as Record<string, unknown>;
    const previewEntries = Object.entries(parsed)
      .slice(0, 2)
      .map(([key, value]) => `${key}=${compactJsonSnippet(JSON.stringify(value))}`);
    argsPreview = previewEntries.join(', ');
  } catch {
    argsPreview = compactJsonSnippet(rawArgs);
  }

  if (argsPreview.length > 72) {
    argsPreview = `${argsPreview.slice(0, 69)}...`;
  }

  return `${toolName}(${argsPreview}) · ${status}`;
}

function roleFromHeader(header: string): ChatMessageRole {
  if (header.includes('[User]')) {
    return 'user';
  }

  if (header.includes('[System]') || header.includes('[Tool]')) {
    return 'system';
  }

  return 'assistant';
}

function parseToolMessage(header: string, body: string, index: number): ChatMessage {
  const summary = summarizeToolHeader(header, body);

  return {
    id: `timeline-${index}`,
    role: 'system',
    kind: 'tool',
    expandable: true,
    summary,
    details: body,
    content: [summary, body].filter(Boolean).join('\n'),
  };
}

function parseRegularMessage(header: string, body: string, index: number): ChatMessage {
  return {
    id: `timeline-${index}`,
    role: roleFromHeader(header),
    kind: 'message',
    content: body,
  };
}

export function parseTimelineText(timelineText: string): ChatMessage[] {
  const lines = timelineText.split('\n');
  const messages: ChatMessage[] = [];

  let activeHeader: string | null = null;
  let activeContent: string[] = [];

  const flush = () => {
    if (!activeHeader) {
      return;
    }

    const normalizedBody = activeContent.map((line) => line.replace(/^\s{2}/, '')).join('\n').trim();
    if (activeHeader.startsWith('[Tool]')) {
      messages.push(parseToolMessage(activeHeader, normalizedBody, messages.length));
    } else if (normalizedBody.length > 0) {
      messages.push(parseRegularMessage(activeHeader, normalizedBody, messages.length));
    }

    activeHeader = null;
    activeContent = [];
  };

  for (const line of lines) {
    if (HEADER_PATTERN.test(line)) {
      flush();
      activeHeader = line.trim();
      continue;
    }

    if (!activeHeader) {
      continue;
    }

    activeContent.push(line);
  }

  flush();

  return messages;
}

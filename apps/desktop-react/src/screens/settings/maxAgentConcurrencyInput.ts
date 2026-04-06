export function parseRequestedMaxAgentConcurrency(input: string): number | null {
  const trimmed = input.trim();

  if (!/^-?\d+$/.test(trimmed)) {
    return null;
  }

  const parsed = Number(trimmed);

  return Number.isSafeInteger(parsed) ? parsed : null;
}

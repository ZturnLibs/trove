/** Convert a KeyboardEvent into the stored shortcut format (e.g. Command+Shift+Space). */
export function eventToShortcutString(event: KeyboardEvent): string | null {
  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.platform);

  const parts: string[] = [];
  if (isMac) {
    if (event.metaKey) parts.push("Command");
    if (event.ctrlKey) parts.push("Ctrl");
  } else if (event.ctrlKey) {
    parts.push("Ctrl");
  } else if (event.metaKey) {
    parts.push("Command");
  }
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");

  const raw = event.key;
  if (
    raw === "Meta" ||
    raw === "Control" ||
    raw === "Shift" ||
    raw === "Alt" ||
    raw === "OS"
  ) {
    return null;
  }

  let keyLabel: string;
  if (raw === " ") keyLabel = "Space";
  else if (raw === "Escape") keyLabel = "Esc";
  else if (raw.length === 1) keyLabel = raw.toUpperCase();
  else keyLabel = raw;

  if (parts.length === 0) return null;

  const candidate = [...parts, keyLabel].join("+");
  const blocked = new Set([
    "Command+Q",
    "Ctrl+Q",
    "Command+W",
    "Ctrl+W",
    "Alt+F4",
    "Command+M",
    "Ctrl+M",
  ]);
  if (blocked.has(candidate)) return null;

  return candidate;
}

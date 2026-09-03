/** Convert a KeyboardEvent into the stored shortcut format (e.g. Command+Alt+Space). */
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

  // Windows / Linux：metaKey 即 Win/Super 键。只含 Win 的组合会劫持系统键
  // （Win+E 资源管理器、Win+D 显示桌面等），必须搭配 Ctrl 或 Alt 才允许。
  if (
    !isMac &&
    parts.includes("Command") &&
    !parts.includes("Ctrl") &&
    !parts.includes("Alt")
  ) {
    return null;
  }

  const candidate = [...parts, keyLabel].join("+");
  // 系统保留键（跨平台）：退出 / 关窗 / 最小化、Spotlight 与输入法开关、
  // 表情符号面板、系统截图、窗口菜单与任务切换、强制退出、任务管理器。
  const blocked = new Set([
    "Command+Q",
    "Ctrl+Q",
    "Command+W",
    "Ctrl+W",
    "Alt+F4",
    "Command+M",
    "Ctrl+M",
    "Command+Space",
    "Ctrl+Space",
    "Command+Ctrl+Space",
    "Command+Shift+3",
    "Command+Shift+4",
    "Command+Shift+5",
    "Command+Alt+Esc",
    "Alt+Tab",
    "Alt+Space",
    "Ctrl+Shift+Esc",
  ]);
  if (blocked.has(candidate)) return null;

  return candidate;
}

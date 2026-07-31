/** Format stored shortcut strings (e.g. Command+Shift+Space) for UI hints. */
export function formatShortcutLabel(raw: string | undefined | null): string {
  if (!raw) return "";
  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.platform);

  return raw
    .split("+")
    .map((part) => {
      const key = part.trim().toLowerCase();
      switch (key) {
        case "command":
        case "cmd":
        case "super":
        case "meta":
          return isMac ? "⌘" : "Ctrl";
        case "control":
        case "ctrl":
          return isMac ? "⌃" : "Ctrl";
        case "alt":
        case "option":
          return isMac ? "⌥" : "Alt";
        case "shift":
          return isMac ? "⇧" : "Shift";
        case "enter":
        case "return":
          return "Enter";
        case "escape":
        case "esc":
          return "Esc";
        case "space":
          return "Space";
        case "backspace":
          return isMac ? "⌫" : "Backspace";
        default:
          if (part.length === 1) return part.toUpperCase();
          return part;
      }
    })
    .join(isMac ? "" : "+");
}

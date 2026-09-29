export const isMacOS = /Macintosh|Mac OS X/i.test(navigator.userAgent);

export const platformName = isMacOS ? "macOS" : "Windows";
export const defaultHotkey = isMacOS ? "Cmd+Shift+Space" : "Ctrl+Shift+Space";
export const holdHotkeyLabel = isMacOS ? "⌘ + Shift + Space" : "Ctrl + Shift + Space";
export const toggleHotkeyLabel = isMacOS
  ? "⌘ + Shift + Option"
  : "Ctrl + Shift + Alt";
export const pasteShortcutLabel = isMacOS ? "⌘V" : "Ctrl+V";

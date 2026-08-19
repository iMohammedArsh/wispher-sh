$ErrorActionPreference = 'Stop'

# This process lives on the Windows host. It can therefore observe and suppress shortcuts while
# Chrome, Word, or another Windows application has focus; WSL/X11 hooks cannot do that.
$source = @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

public static class WisperHotkeys {
  private const int WH_KEYBOARD_LL = 13;
  private const int WM_KEYDOWN = 0x0100;
  private const int WM_KEYUP = 0x0101;
  private const int WM_SYSKEYDOWN = 0x0104;
  private const int WM_SYSKEYUP = 0x0105;
  private const int VK_SPACE = 0x20;
  private const int VK_CONTROL = 0x11;
  private const int VK_SHIFT = 0x10;
  private const int VK_MENU = 0x12;
  private static bool spaceIsDown;
  private static bool toggleShortcut;
  private static HookProc callback = HookCallback;

  private delegate IntPtr HookProc(int code, IntPtr message, IntPtr data);

  [StructLayout(LayoutKind.Sequential)]
  private struct KBDLLHOOKSTRUCT {
    public uint vkCode;
    public uint scanCode;
    public uint flags;
    public uint time;
    public IntPtr dwExtraInfo;
  }

  [DllImport("user32.dll", SetLastError=true)]
  private static extern IntPtr SetWindowsHookEx(int idHook, HookProc procedure, IntPtr module, uint threadId);
  [DllImport("user32.dll", SetLastError=true)]
  private static extern bool UnhookWindowsHookEx(IntPtr hook);
  [DllImport("user32.dll")]
  private static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr message, IntPtr data);
  [DllImport("user32.dll")]
  private static extern short GetAsyncKeyState(int key);
  [DllImport("user32.dll")]
  private static extern int GetMessage(out NativeMessage message, IntPtr window, uint min, uint max);
  [DllImport("kernel32.dll")]
  private static extern IntPtr GetModuleHandle(string moduleName);

  [StructLayout(LayoutKind.Sequential)]
  private struct NativeMessage {
    public IntPtr hwnd;
    public uint message;
    public IntPtr wParam;
    public IntPtr lParam;
    public uint time;
    public int ptX;
    public int ptY;
  }

  private static bool IsDown(int key) { return (GetAsyncKeyState(key) & 0x8000) != 0; }

  private static IntPtr HookCallback(int code, IntPtr message, IntPtr data) {
    if (code >= 0) {
      var key = (KBDLLHOOKSTRUCT)Marshal.PtrToStructure(data, typeof(KBDLLHOOKSTRUCT));
      var eventType = message.ToInt32();
      var isDown = eventType == WM_KEYDOWN || eventType == WM_SYSKEYDOWN;
      var isUp = eventType == WM_KEYUP || eventType == WM_SYSKEYUP;
      if (key.vkCode == VK_SPACE && isDown && !spaceIsDown && IsDown(VK_CONTROL) && IsDown(VK_SHIFT)) {
        // Ctrl+Shift+Space is push-to-talk. Adding Alt enables hands-free mode.
        // This avoids Windows' Alt+Space system menu entirely.
        toggleShortcut = IsDown(VK_MENU);
        spaceIsDown = true;
        Console.WriteLine(toggleShortcut ? "toggle" : "push_down");
        return (IntPtr)1;
      }
      // Key release must not depend on Alt or Ctrl still being down. Windows can
      // deliver modifier release before Space release, otherwise dictation sticks.
      if (key.vkCode == VK_SPACE && isUp && spaceIsDown) {
        if (!toggleShortcut) Console.WriteLine("push_up");
        toggleShortcut = false;
        spaceIsDown = false;
        return (IntPtr)1;
      }
    }
    return CallNextHookEx(IntPtr.Zero, code, message, data);
  }

  public static void Run() {
    var hook = SetWindowsHookEx(WH_KEYBOARD_LL, callback, GetModuleHandle(null), 0);
    if (hook == IntPtr.Zero) throw new InvalidOperationException("Windows rejected the background keyboard hook.");
    Console.WriteLine("ready");
    NativeMessage message;
    while (GetMessage(out message, IntPtr.Zero, 0, 0) > 0) { }
    UnhookWindowsHookEx(hook);
  }
}
'@

Add-Type -TypeDefinition $source
[WisperHotkeys]::Run()

import React from "react";
import { ShieldCheck, Monitor, Command, CheckCircle2 } from "lucide-react";

export const AccessibilityGuide: React.FC = () => {
  return (
    <div className="glass-panel rounded-3xl p-6 border border-slate-800/80 shadow-2xl space-y-6">
      {/* Header */}
      <div className="border-b border-slate-800/80 pb-4">
        <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
          <ShieldCheck className="w-4 h-4 text-emerald-400" />
          OS Accessibility & Keystroke Injection Guide
        </h3>
        <p className="text-xs text-slate-400">
          Wisper requires low-level system permissions to detect background hotkeys and inject text into target apps.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 text-xs text-slate-300">
        {/* Windows 10 / 11 Card */}
        <div className="bg-slate-950/60 p-5 rounded-2xl border border-slate-800/80 space-y-4 shadow-lg">
          <div className="flex items-center gap-2 text-slate-100 font-bold text-sm border-b border-slate-800/60 pb-3">
            <Monitor className="w-4 h-4 text-blue-400" />
            <span>Windows 11 / 10 Setup</span>
          </div>

          <ul className="space-y-3 text-slate-400 leading-relaxed">
            <li className="flex items-start gap-2.5">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
              <span>
                <strong className="text-slate-200">Global Hotkeys:</strong> Low-level keyboard hook (<code className="text-purple-300 font-mono">rdev</code>) works automatically across all apps.
              </span>
            </li>
            <li className="flex items-start gap-2.5">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
              <span>
                <strong className="text-slate-200">Keystroke Injection:</strong> Synthetic paste (<code className="text-purple-300 font-mono">Ctrl+V</code> via <code className="text-slate-200">enigo</code>) pastes text directly into focused text inputs.
              </span>
            </li>
            <li className="flex items-start gap-2.5">
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
              <span>
                <strong className="text-slate-200">Elevated Apps:</strong> To dictate into Administrator/Elevated applications (e.g. Admin Command Prompt or Windows Terminal), launch Wisper as Administrator.
              </span>
            </li>
          </ul>
        </div>

        {/* macOS Setup Card */}
        <div className="bg-slate-950/60 p-5 rounded-2xl border border-slate-800/80 space-y-4 shadow-lg">
          <div className="flex items-center gap-2 text-slate-100 font-bold text-sm border-b border-slate-800/60 pb-3">
            <Command className="w-4 h-4 text-purple-400" />
            <span>macOS System Settings Setup</span>
          </div>

          <ol className="list-decimal list-inside space-y-3 text-slate-400 leading-relaxed">
            <li>
              Open <strong className="text-slate-200">System Settings → Privacy & Security</strong>.
            </li>
            <li>
              Navigate to <strong className="text-slate-200">Accessibility</strong> and grant permission to <strong className="text-slate-200">Wisper Assistant</strong> so it can simulate <code className="text-purple-300 font-mono">Cmd+V</code> keystrokes.
            </li>
            <li>
              Navigate to <strong className="text-slate-200">Input Monitoring</strong> and toggle ON for background hotkey detection.
            </li>
          </ol>
        </div>
      </div>
    </div>
  );
};

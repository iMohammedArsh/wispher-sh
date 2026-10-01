import React, { useState } from "react";
import {
  Settings,
  Key,
  Volume2,
  Clipboard,
  Cpu,
  Keyboard,
  Eye,
  EyeOff,
  CheckCircle2,
} from "lucide-react";
import { AppSettings } from "../types";
import { holdHotkeyLabel, pasteShortcutLabel, toggleHotkeyLabel } from "../platform";

interface SettingsPanelProps {
  settings: AppSettings;
  onUpdateSettings: (settings: AppSettings) => void;
  onSave: () => void;
  savedSuccess: boolean;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({
  settings,
  onUpdateSettings,
  onSave,
  savedSuccess,
}) => {
  const [showApiKey, setShowApiKey] = useState(false);

  return (
    <div className="glass-panel rounded-3xl p-6 border border-slate-800/80 shadow-2xl space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-slate-800/80 pb-4">
        <div>
          <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
            <Settings className="w-4 h-4 text-purple-400" />
            Engine & Hardware Configuration
          </h3>
          <p className="text-xs text-slate-400">
            Configure Gemini 3.5 transcription, Gemini 3.1 formatting, audio gain, hotkeys, and delivery.
          </p>
        </div>

        <button
          onClick={onSave}
          className="flex items-center justify-center gap-2 px-5 py-2.5 rounded-xl bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs transition-all shadow-md shadow-purple-950 cursor-pointer"
        >
          <CheckCircle2 className="w-4 h-4" />
          {savedSuccess ? "Saved Settings!" : "Save Settings"}
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Section 1: Gemini AI Key & Model */}
        <div className="bg-slate-950/50 border border-slate-800/80 rounded-2xl p-5 space-y-4">
          <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-2">
            <Key className="w-4 h-4 text-purple-400" />
            AI & Semantic Processing
          </h4>

          {/* Gemini API Key */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Gemini API Key
            </label>
            <div className="relative">
              <input
                type={showApiKey ? "text" : "password"}
                value={settings.gemini_api_key}
                onChange={(e) =>
                  onUpdateSettings({ ...settings, gemini_api_key: e.target.value })
                }
                placeholder="AIzaSy..."
                className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 pr-10 font-mono"
              />
              <button
                type="button"
                onClick={() => setShowApiKey(!showApiKey)}
                className="absolute right-3 top-2.5 text-slate-400 hover:text-slate-200 cursor-pointer"
              >
                {showApiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
            <p className="text-[11px] text-slate-500">
              When configured, audio is temporarily uploaded to Gemini 3.5 Transcribe and deleted after processing. Bundled local Whisper remains the automatic offline fallback.
            </p>
          </div>

          {/* Gemini Model Selector */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Gemini 3.1 Formatting Model
            </label>
            <select
              value={settings.gemini_model || "gemini-3.1-flash-lite"}
              onChange={(e) =>
                onUpdateSettings({ ...settings, gemini_model: e.target.value })
              }
              className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 cursor-pointer"
            >
              <option value="gemini-3.1-flash-lite">Gemini 3.1 Flash-Lite (Fast — Recommended)</option>
              <option value="gemini-3.1-pro-preview">Gemini 3.1 Pro Preview (Higher quality)</option>
            </select>
            <p className="text-[11px] text-slate-500">
              Speech recognition always uses Gemini 3.5 Transcribe when a key is available. This model cleans the transcript and creates class notes.
            </p>
          </div>
        </div>

        {/* Section 2: Audio Gain & Microphone Boost */}
        <div className="bg-slate-950/50 border border-slate-800/80 rounded-2xl p-5 space-y-4">
          <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-2">
            <Volume2 className="w-4 h-4 text-cyan-400" />
            Microphone & Audio Gain
          </h4>

          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <label className="text-xs font-medium text-slate-300">
                Microphone Gain Boost
              </label>
              <span className="text-xs font-mono text-purple-400 font-bold bg-purple-950/60 px-2 py-0.5 rounded border border-purple-800/40">
                {settings.audio_gain_factor}x
              </span>
            </div>
            <input
              type="range"
              min="1.0"
              max="8.0"
              step="0.5"
              value={settings.audio_gain_factor}
              onChange={(e) =>
                onUpdateSettings({
                  ...settings,
                  audio_gain_factor: parseFloat(e.target.value),
                })
              }
              className="w-full accent-purple-500 bg-slate-900 cursor-pointer"
            />
            <p className="text-[11px] text-slate-500 leading-relaxed">
              Sets the maximum adaptive boost before transcription. Ultra-quiet whispers are lifted automatically; increase this only if your headset is still unusually quiet.
            </p>
          </div>
        </div>

        {/* Section 3: Delivery & Hotkeys */}
        <div className="bg-slate-950/50 border border-slate-800/80 rounded-2xl p-5 space-y-4">
          <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-2">
            <Clipboard className="w-4 h-4 text-pink-400" />
            Dictation Delivery & Shortcuts
          </h4>

          {/* Delivery Mode */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Dictation Delivery Method
            </label>
            <select
              value={settings.delivery_mode}
              onChange={(e) =>
                onUpdateSettings({ ...settings, delivery_mode: e.target.value })
              }
              className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 cursor-pointer"
            >
              <option value="copy_and_paste">Copy & Paste (Recommended)</option>
              <option value="copy_only">Copy to Clipboard Only</option>
              <option value="paste_and_restore">Paste & Restore Clipboard History</option>
            </select>
            <p className="text-[11px] text-slate-500">
              Automatically pastes text into the active application using the standard {pasteShortcutLabel} shortcut.
            </p>
          </div>

          {/* Global Hotkeys Summary */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300 flex items-center gap-1.5">
              <Keyboard className="w-3.5 h-3.5 text-indigo-400" />
              Active System Hotkeys
            </label>
            <div className="bg-slate-900/90 border border-slate-800 rounded-xl p-3 text-xs font-mono space-y-1 text-slate-300">
              <div className="flex justify-between">
                <span>Hold to talk:</span>
                <span className="text-purple-300 font-bold">{holdHotkeyLabel}</span>
              </div>
              <div className="flex justify-between">
                <span>Hands-free toggle:</span>
                <span className="text-indigo-300 font-bold">{toggleHotkeyLabel}</span>
              </div>
            </div>
          </div>

          <div className="space-y-2 border-t border-slate-800/80 pt-4">
            <label className="flex items-start gap-3 text-xs text-slate-300 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.app_usage_tracking}
                onChange={(e) => onUpdateSettings({ ...settings, app_usage_tracking: e.target.checked })}
                className="mt-0.5 h-4 w-4 accent-purple-500"
              />
              <span>
                <strong className="block font-medium">Local app-category insights</strong>
                <span className="mt-1 block text-[11px] text-slate-500">Record only the foreground app name, such as VS Code or Outlook, to group dictation locally. Hams never stores window titles, filenames, screen content, or typed text.</span>
              </span>
            </label>
          </div>
        </div>

        {/* Section 4: Whisper Local Engine Paths */}
        <div className="bg-slate-950/50 border border-slate-800/80 rounded-2xl p-5 space-y-4">
          <h4 className="text-xs font-bold text-slate-200 uppercase tracking-wider flex items-center gap-2">
            <Cpu className="w-4 h-4 text-emerald-400" />
            Offline Whisper Fallback
          </h4>

          {/* Whisper Model Path */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Dictation Language
            </label>
            <select
              value={settings.transcription_language}
              onChange={(e) =>
                onUpdateSettings({
                  ...settings,
                  transcription_language: e.target.value as AppSettings["transcription_language"],
                })
              }
              className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 cursor-pointer"
            >
              <option value="auto">Auto-detect English / Malayalam</option>
              <option value="en">English</option>
              <option value="ml">Malayalam</option>
            </select>
            <p className="text-[11px] text-slate-500">
              Choose Malayalam for the most reliable Malayalam script. Auto is best when you mix English and Malayalam.
            </p>
          </div>

          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Whisper Model (.bin) Path
            </label>
            <input
              type="text"
              value={settings.whisper_model_path}
              onChange={(e) =>
                onUpdateSettings({ ...settings, whisper_model_path: e.target.value })
              }
              placeholder="ggml-base.en.bin"
              className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 font-mono"
            />
          </div>

          {/* Whisper Binary Path */}
          <div className="space-y-2">
            <label className="text-xs font-medium text-slate-300">
              Whisper Executable Binary Path
            </label>
            <input
              type="text"
              value={settings.whisper_binary_path}
              onChange={(e) =>
                onUpdateSettings({ ...settings, whisper_binary_path: e.target.value })
              }
              placeholder="whisper.cpp executable"
              className="w-full bg-slate-900 border border-slate-700/80 rounded-xl px-3.5 py-2.5 text-xs text-slate-200 focus:outline-none focus:border-purple-500 font-mono"
            />
          </div>
        </div>
      </div>
    </div>
  );
};
